use crate::config::LlmConfig;
use crate::error::{Error, Result};
use serde_json::{Value, json};
use std::time::Duration;

const MAX_RETRIES: usize = 3;

/// Script system prompt ported from MoneyPrinterTurbo (app/services/llm.py).
pub const SCRIPT_SYSTEM_PROMPT: &str = r#"# Role: Video Script Generator

## Goals:
Generate a script for a video, depending on the subject of the video.

## Constrains:
1. the script is to be returned as a string with the specified number of paragraphs.
2. do not under any circumstance reference this prompt in your response.
3. get straight to the point, don't start with unnecessary things like, "welcome to this video".
4. you must not include any type of markdown or formatting in the script, never use a title.
5. only return the raw content of the script.
6. do not include "voiceover", "narrator" or similar indicators of what should be spoken at the beginning of each paragraph or line.
7. you must not mention the prompt, or anything about the script itself. also, never talk about the amount of paragraphs or lines. just write the script.
8. respond in the same language as the video subject."#;

pub struct LlmClient {
    config: LlmConfig,
    http: reqwest::blocking::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        Ok(Self { config, http })
    }

    /// One chat-completions round trip against any OpenAI-compatible server.
    pub fn chat(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let body = json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": prompt}],
        });

        let mut last_error = String::new();
        for attempt in 1..=MAX_RETRIES {
            let mut request = self.http.post(&url).json(&body);
            if !self.config.api_key.trim().is_empty() {
                request = request.bearer_auth(self.config.api_key.trim());
            }
            match request.send() {
                Ok(response) => {
                    let status = response.status();
                    let value: Value = match response.json() {
                        Ok(value) => value,
                        Err(e) => {
                            last_error = format!("invalid JSON response: {e}");
                            continue;
                        }
                    };
                    if !status.is_success() {
                        last_error = format!(
                            "HTTP {status}: {}",
                            value["error"]["message"].as_str().unwrap_or("unknown error")
                        );
                        continue;
                    }
                    let content = value["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or_default();
                    let cleaned = strip_think_blocks(content).trim().to_string();
                    if cleaned.is_empty() {
                        last_error = "empty completion content".to_string();
                        continue;
                    }
                    return Ok(cleaned);
                }
                Err(e) => last_error = e.to_string(),
            }
            if attempt < MAX_RETRIES {
                eprintln!("LLM request failed (attempt {attempt}/{MAX_RETRIES}): {last_error}");
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        Err(Error::Llm(format!(
            "{last_error} (base_url: {}, model: {})",
            self.config.base_url, self.config.model
        )))
    }

    pub fn generate_script(
        &self,
        subject: &str,
        language: &str,
        paragraphs: u32,
    ) -> Result<String> {
        let paragraphs = paragraphs.clamp(1, 10);
        let mut prompt = format!(
            "{SCRIPT_SYSTEM_PROMPT}\n\n# Initialization:\n- video subject: {subject}\n- number of paragraphs: {paragraphs}"
        );
        if !language.trim().is_empty() {
            prompt.push_str(&format!("\n- language: {}", language.trim()));
        }
        let response = self.chat(&prompt)?;
        let script = clean_script(&response);
        if script.is_empty() {
            return Err(Error::Llm("LLM returned an empty script".to_string()));
        }
        Ok(script)
    }

    /// Generate English stock-footage search terms as a JSON array of strings.
    /// With `ordered`, terms follow the script's narrative order so sequential
    /// material matching keeps visuals in sync with narration.
    pub fn generate_terms(
        &self,
        subject: &str,
        script: &str,
        amount: usize,
        ordered: bool,
    ) -> Result<Vec<String>> {
        let ordering_rule = if ordered {
            "\n6. keep the terms in the same order as the script narration; earlier terms must describe earlier visual moments."
        } else {
            ""
        };
        let prompt = format!(
            r#"# Role: Video Search Terms Generator

## Goals:
Generate {amount} search terms for stock videos, depending on the subject of a video.

## Constrains:
1. the search terms are to be returned as a json-array of strings.
2. each search term should consist of 1-3 words, always add the main subject of the video.
3. you must only return the json-array of strings. you must not return anything else. you must not return the script.
4. the search terms must be related to the subject of the video.
5. reply with english search terms only.{ordering_rule}

## Output Example:
["search term 1", "search term 2", "search term 3", "search term 4", "search term 5"]

## Context:
### Video Subject
{subject}

### Video Script
{script}

Please note that you must use English for generating video search terms; Chinese is not accepted."#
        );

        let response = self.chat(&prompt)?;
        let terms = parse_string_array(&response)
            .ok_or_else(|| Error::Llm(format!("could not parse search terms from: {response}")))?;
        if terms.is_empty() {
            return Err(Error::Llm("LLM returned no search terms".to_string()));
        }
        Ok(terms)
    }

    /// Title/caption/hashtags for a target platform. Limits ported from
    /// MoneyPrinterTurbo's SOCIAL_PLATFORMS table.
    pub fn generate_social_metadata(
        &self,
        subject: &str,
        script: &str,
        platform: &str,
    ) -> Result<SocialMetadata> {
        let limits = PlatformLimits::for_platform(platform);
        let prompt = format!(
            r#"# Role: Short-Form Social Media Copywriter

## Goals:
Write publishing metadata for a {label} post of a short vertical video.

## Constrains:
1. return only a JSON object with keys "title", "caption", "hashtags".
2. "title" must be at most {title_max} characters, no hashtags inside.
3. "caption" must be at most {caption_max} characters and may include a call to action.
4. "hashtags" must be a JSON array of exactly {hashtag_count} strings, each starting with '#'.
5. write in the same language as the video script.
6. never mention this prompt.

## Context:
### Video Subject
{subject}

### Video Script
{script}"#,
            label = limits.label,
            title_max = limits.title_max,
            caption_max = limits.caption_max,
            hashtag_count = limits.hashtag_count,
        );
        let response = self.chat(&prompt)?;
        let value: Value = serde_json::from_str(strip_code_fence(&response))
            .or_else(|_| extract_json_object(&response))
            .map_err(|_| Error::Llm(format!("could not parse social metadata: {response}")))?;
        Ok(SocialMetadata {
            title: truncate_chars(
                value["title"].as_str().unwrap_or(subject),
                limits.title_max,
            ),
            caption: truncate_chars(
                value["caption"].as_str().unwrap_or_default(),
                limits.caption_max,
            ),
            hashtags: value["hashtags"]
                .as_array()
                .map(|tags| {
                    tags.iter()
                        .filter_map(|tag| tag.as_str())
                        .map(normalize_hashtag)
                        .take(limits.hashtag_count)
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SocialMetadata {
    pub title: String,
    pub caption: String,
    pub hashtags: Vec<String>,
}

pub struct PlatformLimits {
    pub label: &'static str,
    pub title_max: usize,
    pub caption_max: usize,
    pub hashtag_count: usize,
}

impl PlatformLimits {
    pub fn for_platform(platform: &str) -> Self {
        match platform.trim().to_lowercase().as_str() {
            "youtube" | "youtube_shorts" => Self {
                label: "YouTube Shorts",
                title_max: 100,
                caption_max: 5000,
                hashtag_count: 3,
            },
            "instagram" | "instagram_reels" => Self {
                label: "Instagram Reels",
                title_max: 125,
                caption_max: 2200,
                hashtag_count: 8,
            },
            "facebook" | "facebook_reels" => Self {
                label: "Facebook Reels",
                title_max: 125,
                caption_max: 2200,
                hashtag_count: 5,
            },
            _ => Self {
                label: "TikTok",
                title_max: 100,
                caption_max: 2200,
                hashtag_count: 5,
            },
        }
    }
}

/// Local reasoning models (Qwen, DeepSeek) emit <think>...</think> blocks that
/// must never reach the narration script.
pub fn strip_think_blocks(text: &str) -> String {
    let closed = regex::RegexBuilder::new(r"<think\b[^>]*>.*?</think>")
        .dot_matches_new_line(true)
        .case_insensitive(true)
        .build()
        .expect("static regex");
    let unclosed = regex::RegexBuilder::new(r"<think\b[^>]*>.*$")
        .dot_matches_new_line(true)
        .case_insensitive(true)
        .build()
        .expect("static regex");
    let text = closed.replace_all(text, "");
    unclosed.replace_all(&text, "").to_string()
}

/// Remove markdown noise the model may add despite instructions.
pub fn clean_script(response: &str) -> String {
    let response = response.replace(['*', '#'], "");
    // Non-greedy so each bracket/paren group is removed independently.
    let brackets = regex::Regex::new(r"\[.*?\]").expect("static regex");
    let parens = regex::Regex::new(r"\(.*?\)").expect("static regex");
    let response = brackets.replace_all(&response, "");
    let response = parens.replace_all(&response, "");
    response.trim().to_string()
}

pub fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .split_once('\n')
        .map(|(_, body)| body)
        .unwrap_or(rest);
    rest.trim_end_matches('`').trim()
}

/// Parse a JSON string array, tolerating code fences and surrounding prose.
pub fn parse_string_array(response: &str) -> Option<Vec<String>> {
    let attempt = |text: &str| -> Option<Vec<String>> {
        let value: Value = serde_json::from_str(text).ok()?;
        let items = value.as_array()?;
        let terms: Vec<String> = items
            .iter()
            .filter_map(|item| item.as_str())
            .map(|term| term.trim().to_string())
            .filter(|term| !term.is_empty())
            .collect();
        (terms.len() == items.len()).then_some(terms)
    };

    attempt(strip_code_fence(response)).or_else(|| {
        let array = regex::RegexBuilder::new(r"\[.*\]")
            .dot_matches_new_line(true)
            .build()
            .expect("static regex");
        attempt(array.find(response)?.as_str())
    })
}

fn extract_json_object(response: &str) -> std::result::Result<Value, serde_json::Error> {
    let object = regex::RegexBuilder::new(r"\{.*\}")
        .dot_matches_new_line(true)
        .build()
        .expect("static regex");
    let text = object
        .find(response)
        .map(|found| found.as_str())
        .unwrap_or(response);
    serde_json::from_str(text)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.trim().chars().take(max_chars).collect()
}

fn normalize_hashtag(tag: &str) -> String {
    let tag = tag.trim().trim_start_matches('#');
    format!("#{}", tag.replace(char::is_whitespace, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_closed_and_unclosed_think_blocks() {
        assert_eq!(
            strip_think_blocks("<think>internal</think>Hello").trim(),
            "Hello"
        );
        assert_eq!(strip_think_blocks("Hi<think>never closed").trim(), "Hi");
    }

    #[test]
    fn cleans_markdown_from_script() {
        assert_eq!(
            clean_script("**Bold** script [note] here (aside)"),
            "Bold script  here"
        );
    }

    #[test]
    fn parses_terms_from_fenced_json() {
        let response = "```json\n[\"ocean waves\", \"coral reef\"]\n```";
        assert_eq!(
            parse_string_array(response).unwrap(),
            vec!["ocean waves", "coral reef"]
        );
    }

    #[test]
    fn recovers_terms_from_prose_wrapped_json() {
        let response = "Sure! Here are the terms:\n[\"city lights\", \"night traffic\"]\nEnjoy!";
        assert_eq!(
            parse_string_array(response).unwrap(),
            vec!["city lights", "night traffic"]
        );
    }

    #[test]
    fn rejects_non_string_arrays() {
        assert!(parse_string_array("[1, 2, 3]").is_none());
    }

    #[test]
    fn hashtags_are_normalized() {
        assert_eq!(normalize_hashtag("  fyp"), "#fyp");
        assert_eq!(normalize_hashtag("#two words"), "#twowords");
    }

    #[test]
    fn platform_limits_default_to_tiktok() {
        assert_eq!(PlatformLimits::for_platform("unknown").label, "TikTok");
        assert_eq!(
            PlatformLimits::for_platform("youtube_shorts").hashtag_count,
            3
        );
    }
}
