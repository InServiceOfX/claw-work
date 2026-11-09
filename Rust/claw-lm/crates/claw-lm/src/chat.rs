use anyhow::{Context, Result, bail, ensure};
use minijinja::{
    Environment, Error, ErrorKind, context,
    value::{Value as TemplateValue, ValueKind},
};
use serde_json::{Value, json};
use std::{fs, path::Path};
use tokenizers::Tokenizer;

fn python_json(value: &Value) -> String {
    match value {
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_json)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(k, v)| format!("{}: {}", serde_json::to_string(k).unwrap(), python_json(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => serde_json::to_string(value).unwrap(),
    }
}
// The public template renders tool results in order, without printing call IDs.
// Canonicalize each result block by its preceding calls so association survives
// completion-order differences in clients that execute calls concurrently.
fn order_tool_results(messages: &mut [Value]) -> Result<()> {
    let mut index = 0;
    while index < messages.len() {
        if messages[index]["role"].as_str() == Some("tool") {
            bail!("Tool result has no preceding assistant calls");
        }
        let ids = messages[index]
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|calls| {
                calls
                    .iter()
                    .map(|c| {
                        c.get("id")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .context("Assistant tool call requires an id")
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        if ids.is_empty() {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        let mut results = std::collections::BTreeMap::new();
        while end < messages.len() && messages[end]["role"].as_str() == Some("tool") {
            let id = messages[end]["tool_call_id"]
                .as_str()
                .context("Tool result requires tool_call_id")?;
            ensure!(
                ids.iter().any(|known| known == id),
                "Unknown tool result id"
            );
            ensure!(
                results
                    .insert(id.to_string(), messages[end].clone())
                    .is_none(),
                "Duplicate tool result id"
            );
            end += 1;
        }
        ensure!(
            results.len() == ids.len(),
            "Missing or duplicate assistant tool results"
        );
        let mut ordered = Vec::new();
        for id in ids {
            ordered.push(
                results
                    .remove(&id)
                    .context("Duplicate assistant tool call id")?,
            );
        }
        messages[index + 1..end].clone_from_slice(&ordered);
        index = end;
    }
    Ok(())
}
pub struct Chat {
    pub tokenizer: Tokenizer,
    environment: Environment<'static>,
}
impl Chat {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self> {
        let directory = directory.as_ref();
        let tokenizer = Tokenizer::from_file(directory.join("tokenizer.json"))
            .map_err(|e| anyhow::anyhow!("Tokenizer: {e}"))?;
        let source = fs::read_to_string(directory.join("chat_template.jinja"))
            .context("Cannot read chat_template.jinja")?;
        let mut environment = Environment::new();
        environment.set_trim_blocks(true);
        environment.set_lstrip_blocks(true);
        environment.set_unknown_method_callback(|state, value, method, args| {
            if value.kind() == ValueKind::Map && method == "items" && args.is_empty() {
                state.apply_filter("items", std::slice::from_ref(value))
            } else if let Some(s) = value.as_str() {
                if method == "split" && args.len() == 1 {
                    let separator = args[0].as_str().ok_or_else(|| {
                        Error::new(ErrorKind::InvalidOperation, "Expected string separator")
                    })?;
                    Ok(TemplateValue::from_serialize(
                        s.split(separator).collect::<Vec<_>>(),
                    ))
                } else {
                    Err(Error::from(ErrorKind::UnknownMethod))
                }
            } else {
                Err(Error::from(ErrorKind::UnknownMethod))
            }
        });
        environment.add_filter(
            "tojson",
            |value: TemplateValue| -> std::result::Result<TemplateValue, Error> {
                let json = serde_json::to_value(value)
                    .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?;
                Ok(TemplateValue::from_safe_string(python_json(&json)))
            },
        );
        environment.add_filter("string", |value: TemplateValue| {
            if value.kind() == ValueKind::Bool {
                if value.is_true() {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            } else if value.is_none() {
                "None".into()
            } else {
                value.to_string()
            }
        });
        environment.add_template_owned("chat", source)?;
        Ok(Self {
            tokenizer,
            environment,
        })
    }
    pub fn render(&self, messages: &[Value], tools: &[Value]) -> Result<String> {
        ensure!(!messages.is_empty(), "Messages must not be empty");
        let mut messages = messages.to_vec();
        for m in &mut messages {
            if let Some(parts) = m.get("content").and_then(Value::as_array) {
                let mut text = String::new();
                for part in parts {
                    ensure!(
                        part["type"].as_str() == Some("text") && part["text"].is_string(),
                        "Only text message blocks are supported"
                    );
                    text.push_str(part["text"].as_str().unwrap());
                }
                m["content"] = Value::String(text);
            }
            let role = m
                .get("role")
                .and_then(Value::as_str)
                .context("Message role is required")?;
            ensure!(
                ["system", "user", "assistant", "tool"].contains(&role),
                "Unsupported message role"
            );
            if let Some(c) = m.get("content") {
                ensure!(
                    c.is_null() || c.is_string(),
                    "Only text message content is supported"
                );
            }
            if role == "tool" {
                ensure!(
                    m.get("tool_call_id").is_some_and(Value::is_string),
                    "Tool result requires tool_call_id"
                );
            }
            if let Some(calls) = m.get_mut("tool_calls") {
                let calls = calls
                    .as_array_mut()
                    .context("tool_calls must be an array")?;
                for call in calls {
                    let arguments = &mut call["function"]["arguments"];
                    if let Some(s) = arguments.as_str() {
                        *arguments =
                            serde_json::from_str(s).context("Invalid JSON tool arguments")?;
                    }
                    ensure!(arguments.is_object(), "Tool arguments must be an object");
                }
            }
        }
        order_tool_results(&mut messages)?;
        for t in tools {
            ensure!(
                t.get("type").and_then(Value::as_str) == Some("function")
                    && t["function"]["name"].is_string()
                    && t["function"]["parameters"].is_object(),
                "Only function tools with JSON parameters are supported"
            );
        }
        Ok(self.environment.get_template("chat")?.render(context!(messages=>messages,tools=>tools,add_generation_prompt=>true,enable_thinking=>false,truncate_history_thinking=>true))?)
    }
    pub fn encode(&self, prompt: &str) -> Result<Vec<u32>> {
        Ok(self
            .tokenizer
            .encode(prompt, false)
            .map_err(|e| anyhow::anyhow!("Encoding: {e}"))?
            .get_ids()
            .to_vec())
    }
    pub fn decode(&self, ids: &[u32]) -> Result<String> {
        self.tokenizer
            .decode(ids, true)
            .map_err(|e| anyhow::anyhow!("Decoding: {e}"))
    }
}
/// Remove only explicitly delimited reasoning; incomplete thought text stays hidden.
pub fn visible_text(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let thought = &rest[start + 7..];
        if let Some(end) = thought.find("</think>") {
            rest = &thought[end + 8..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out.replace("</think>", "")
}
/// Holds partial control/stop markers until their meaning is known.
#[derive(Default)]
pub struct TextStream {
    raw: String,
    sent: usize,
    stops: Vec<String>,
    stopped: bool,
}
impl TextStream {
    pub fn new(stops: Vec<String>) -> Result<Self> {
        ensure!(
            stops.len() <= 16 && stops.iter().all(|s| !s.is_empty() && s.len() <= 256),
            "Invalid stop strings"
        );
        Ok(Self {
            stops,
            ..Self::default()
        })
    }
    pub fn push(&mut self, text: &str, finish: bool) -> Result<(String, bool)> {
        self.raw.push_str(text);
        let mut visible = visible_text(&self.raw);
        if let Some(index) = self.stops.iter().filter_map(|s| visible.find(s)).min() {
            visible.truncate(index);
            self.stopped = true;
        }
        if !finish && !self.stopped {
            let controls = ["<think>", "</think>", "<tool_call>"];
            let held = controls
                .iter()
                .copied()
                .chain(self.stops.iter().map(String::as_str))
                .flat_map(|marker| {
                    (1..marker.len())
                        .filter(|&n| marker.is_char_boundary(n) && visible.ends_with(&marker[..n]))
                        .collect::<Vec<_>>()
                })
                .max()
                .unwrap_or(0);
            visible.truncate(visible.len() - held);
        }
        ensure!(
            self.sent <= visible.len(),
            "Text decoder changed an already emitted prefix"
        );
        let delta = visible[self.sent..].to_string();
        self.sent = visible.len();
        Ok((delta, self.stopped))
    }
}
fn typed_argument(raw: &str, schema: &Value) -> Result<Value> {
    let value = match schema.get("type").and_then(Value::as_str) {
        Some("string") | None => Value::String(raw.to_string()),
        Some("integer") => json!(
            raw.parse::<i64>()
                .context("Invalid integer tool argument")?
        ),
        Some("number") => {
            let x = raw
                .parse::<f64>()
                .context("Invalid numeric tool argument")?;
            ensure!(x.is_finite(), "Nonfinite tool argument");
            json!(x)
        }
        Some("boolean") => json!(match raw {
            "true" | "True" => true,
            "false" | "False" => false,
            _ => bail!("Invalid boolean tool argument"),
        }),
        Some("object" | "array" | "null") => {
            serde_json::from_str(raw).context("Invalid structured tool argument")?
        }
        _ => bail!("Unsupported tool argument schema type"),
    };
    match schema["type"].as_str() {
        Some("object") => ensure!(value.is_object(), "Expected object argument"),
        Some("array") => ensure!(value.is_array(), "Expected array argument"),
        Some("null") => ensure!(value.is_null(), "Expected null argument"),
        _ => {}
    }
    if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
        ensure!(allowed.contains(&value), "Tool argument is outside enum");
    }
    Ok(value)
}
pub fn parse_tool_calls(
    raw: &str,
    tools: &[Value],
    id_prefix: &str,
) -> Result<(String, Vec<Value>)> {
    let visible = visible_text(raw);
    // This checkpoint also emits a compact function block without the outer tag.
    // Normalize only a complete block; all names and arguments are still checked.
    let visible = if !visible.contains("<tool_call>") {
        if let Some(start) = visible.find("<function=") {
            ensure!(
                visible[start..].trim_end().ends_with("</function>"),
                "Incomplete compact tool output"
            );
            format!(
                "{}<tool_call>{}</tool_call>",
                &visible[..start],
                visible[start..].trim_end()
            )
        } else {
            visible
        }
    } else {
        visible
    };
    let mut rest = visible.as_str();
    let mut content = String::new();
    let mut calls = Vec::new();
    while let Some(start) = rest.find("<tool_call>") {
        content.push_str(&rest[..start]);
        let body = &rest[start + 11..];
        let end = body.find("</tool_call>").context("Unclosed tool_call")?;
        let body = body[..end].trim();
        let f = body
            .strip_prefix("<function=")
            .context("Invalid tool function opening")?;
        let split = f.find('>').context("Invalid function name")?;
        let name = &f[..split];
        let tool = tools
            .iter()
            .find(|t| t["function"]["name"].as_str() == Some(name))
            .context("Model requested an unknown tool")?;
        let schema = &tool["function"]["parameters"];
        let mut parameters = f[split + 1..].trim();
        let mut arguments = serde_json::Map::new();
        // An observed no-argument completion uses this empty positional marker.
        if parameters
            .strip_suffix("</function>")
            .is_some_and(|p| p.trim() == "<parameter>[]</parameter>")
        {
            ensure!(
                schema["properties"]
                    .as_object()
                    .is_some_and(|p| p.is_empty())
                    && schema
                        .get("required")
                        .is_none_or(|v| v.as_array().is_some_and(|a| a.is_empty())),
                "Empty marker cannot replace required arguments"
            );
            parameters = "</function>";
        }
        while !parameters.starts_with("</function>") {
            let param = parameters
                .strip_prefix("<parameter=")
                .context("Invalid parameter opening")?;
            let split = param.find('>').context("Invalid parameter name")?;
            let key = &param[..split];
            let value = &param[split + 1..];
            let end = value.find("</parameter>").context("Unclosed parameter")?;
            let field = schema["properties"]
                .get(key)
                .context("Unknown tool argument")?;
            ensure!(!arguments.contains_key(key), "Duplicate tool argument");
            arguments.insert(key.to_string(), typed_argument(value[..end].trim(), field)?);
            parameters = value[end + 12..].trim();
        }
        ensure!(
            parameters == "</function>",
            "Unexpected content after tool function"
        );
        if let Some(required) = schema["required"].as_array() {
            for key in required {
                ensure!(
                    key.as_str().is_some_and(|k| arguments.contains_key(k)),
                    "Missing required tool argument"
                );
            }
        }
        calls.push(json!({"id":format!("call_{id_prefix}_{}",calls.len()),"type":"function","function":{"name":name,"arguments":serde_json::to_string(&arguments)?}}));
        rest = &rest[start + 11 + end + 12..];
    }
    ensure!(
        !rest.contains("<tool_call") && !rest.contains("<function="),
        "Incomplete tool output"
    );
    content.push_str(rest);
    Ok((content.trim().to_string(), calls))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn results_follow_call_ids_and_unknown_ids_fail() {
        let mut messages = vec![
            json!({"role":"assistant","tool_calls":[{"id":"a"},{"id":"b"}]}),
            json!({"role":"tool","tool_call_id":"b","content":"second"}),
            json!({"role":"tool","tool_call_id":"a","content":"first"}),
        ];
        order_tool_results(&mut messages).unwrap();
        assert_eq!(messages[1]["content"], "first");
        assert_eq!(messages[2]["content"], "second");
        messages[1]["tool_call_id"] = json!("unknown");
        assert!(order_tool_results(&mut messages).is_err());
        assert!(order_tool_results(&mut [json!({"role":"tool","tool_call_id":"orphan"})]).is_err());
    }
    #[test]
    fn streaming_hides_split_controls_and_stops() {
        let mut s = TextStream::new(vec!["END".into()]).unwrap();
        let mut result = String::new();
        for part in [
            "Hello <thi",
            "nk>private",
            "</thi",
            "nk> world E",
            "N",
            "D extra",
        ] {
            result.push_str(&s.push(part, false).unwrap().0);
        }
        assert_eq!(result, "Hello  world ");
        assert!(s.stopped);
    }
    #[test]
    fn observed_compact_empty_call_is_checked() {
        let tool = json!({"type":"function","function":{"name":"lookup_code","parameters":{"type":"object","properties":{},"required":[]}}});
        let raw = "<function=lookup_code>\n<parameter>[]</parameter>\n</function>";
        let (_, calls) = parse_tool_calls(raw, &[tool], "test").unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["arguments"], "{}");
        assert!(parse_tool_calls(raw, &[], "test").is_err());
        assert!(parse_tool_calls("<function=lookup_code>", &[], "test").is_err());
    }
    #[test]
    fn unicode_stop_prefix_is_held() {
        let mut s = TextStream::new(vec!["世界".into()]).unwrap();
        assert_eq!(s.push("Hello 世", false).unwrap(), ("Hello ".into(), false));
        assert_eq!(s.push("界 extra", false).unwrap(), ("".into(), true));
    }
    #[test]
    fn typed_tools_and_malformed_rejection() {
        let tools = vec![
            json!({"type":"function","function":{"name":"add","parameters":{"type":"object","properties":{"a":{"type":"integer"},"ok":{"type":"boolean"}},"required":["a","ok"]}}}),
        ];
        let raw = "<tool_call>\n<function=add>\n<parameter=a>\n2\n</parameter>\n<parameter=ok>\ntrue\n</parameter>\n</function>\n</tool_call>";
        let (_, calls) = parse_tool_calls(raw, &tools, "test").unwrap();
        let args: Value =
            serde_json::from_str(calls[0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args, json!({"a":2,"ok":true}));
        assert!(parse_tool_calls("<tool_call>", &tools, "x").is_err());
        assert!(parse_tool_calls(&raw.replace("true", "maybe"), &tools, "x").is_err());
    }
}
