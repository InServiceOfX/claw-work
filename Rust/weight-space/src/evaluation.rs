use crate::{
    hash_bytes, hash_file,
    server::{OwnedServer, ServerConfig, local_client, read_json_response},
    write_json_new,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    time::Instant,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub prompt: String,
    pub expected: Expected,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expected {
    Integer(i64),
    Exact(String),
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub fn synthetic_cases(seed: u64, count: usize) -> Result<Vec<Case>> {
    ensure!((1..=1000).contains(&count), "case count must be 1..1000");
    let mut rng = Rng(seed.max(1));
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for i in 0..count {
        loop {
            let a = (rng.next() % 90 + 10) as i64;
            let b = (rng.next() % 90 + 10) as i64;
            let c = (rng.next() % 8 + 2) as i64;
            let (prompt, expected) = if i % 6 == 5 {
                let tag = format!("TAG{}", rng.next() % 1_000_000_000);
                (
                    format!("Reply with exactly {tag} and nothing else."),
                    Expected::Exact(tag),
                )
            } else {
                let (p, n) = match i % 4 {
                    0 => (format!("Compute ({a} + {b}) * {c}."), (a + b) * c),
                    1 => (
                        format!(
                            "A shop has {a} boxes with {c} pencils each and sells {b} pencils. How many pencils remain?"
                        ),
                        a * c - b,
                    ),
                    2 => (
                        format!("Solve {c}*x + {b} = {} for integer x.", a * c + b),
                        a,
                    ),
                    _ => (
                        format!(
                            "A train carries {a} people. At each of {c} stops, {b} people board. Nobody leaves. How many people are on the train at the end?"
                        ),
                        a + c * b,
                    ),
                };
                (
                    format!(
                        "{p} Please reason step by step, and put your final integer answer within \\boxed{{}}."
                    ),
                    Expected::Integer(n),
                )
            };
            if seen.insert(prompt.clone()) {
                result.push(Case {
                    id: format!("s{seed}-{i:03}"),
                    prompt,
                    expected,
                });
                break;
            }
        }
    }
    Ok(result)
}

pub fn grade(case: &Case, text: &str, thinking: bool) -> bool {
    let answer = if thinking {
        match text.rsplit_once("</think>") {
            Some((_, a)) => a,
            None => return false,
        }
    } else if let Some((_, a)) = text.rsplit_once("</think>") {
        a
    } else {
        if text.contains("<think>") {
            return false;
        }
        text
    };
    match &case.expected {
        Expected::Exact(expected) => answer.trim() == expected,
        Expected::Integer(expected) => {
            let Some((_, tail)) = answer.rsplit_once("\\boxed{") else {
                return false;
            };
            let Some((n, _)) = tail.split_once('}') else {
                return false;
            };
            let n = n.trim();
            let digits = n.strip_prefix(['-', '+']).unwrap_or(n);
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && n.parse::<i64>().is_ok_and(|n| n == *expected)
        }
    }
}

/// Diagnostic: an identical contiguous 32-token n-gram occurs at least 4 times
/// anywhere in a completion of at least 128 tokens. Overlaps count.
pub fn repeated_ngram(tokens: &[u32]) -> bool {
    if tokens.len() < 128 {
        return false;
    }
    let mut counts: HashMap<&[u32], u32> = HashMap::new();
    for window in tokens.windows(32) {
        let n = counts.entry(window).or_default();
        *n += 1;
        if *n >= 4 {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub arm: String,
    pub max_tokens: usize,
    pub thinking: bool,
    pub case_sha256: String,
    pub model_sha256: String,
    #[serde(default)]
    pub tool_sha256: String,
    #[serde(default)]
    pub input_case_file_sha256: String,
    pub seed: u32,
    pub repeat_penalty: f64,
    pub server: ServerConfig,
    pub protocol: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub id: String,
    pub seed: u32,
    pub correct: bool,
    pub thinking_closed: bool,
    pub cap_hit: bool,
    pub repetition: bool,
    pub tokens: usize,
    pub seconds: f64,
    pub error: Option<String>,
    pub response: Option<Value>,
}

pub fn run(
    model: &Path,
    cases_path: &Path,
    out: &Path,
    arm: &str,
    max_tokens: usize,
    thinking: bool,
    cfg: ServerConfig,
) -> Result<Value> {
    crate::ensure_external_output(out)?;
    ensure!(
        ["base", "edit", "decoding", "diagnostic"].contains(&arm),
        "invalid arm"
    );
    ensure!(
        (1..=32768).contains(&max_tokens) && cfg.context >= max_tokens + 512,
        "invalid generation/context budget"
    );
    ensure!(
        fs::metadata(cases_path)?.len() < 4 * 1024 * 1024,
        "case file too large"
    );
    let cases: Vec<Case> = serde_json::from_reader(File::open(cases_path)?)?;
    ensure!(
        !cases.is_empty() && cases.len() <= 1000,
        "invalid case count"
    );
    let mut ids = HashSet::new();
    let mut prompts = HashSet::new();
    ensure!(
        cases.iter().all(|c| !c.id.is_empty()
            && ids.insert(c.id.clone())
            && !c.prompt.is_empty()
            && c.prompt.len() < 65536
            && prompts.insert(c.prompt.clone())),
        "duplicate/invalid cases"
    );
    let canonical = serde_json::to_vec(&cases)?;
    let settings = Settings {
        arm: arm.to_owned(),
        max_tokens,
        thinking,
        case_sha256: hash_bytes(&canonical),
        model_sha256: hash_file(model)?,
        tool_sha256: hash_file(&std::env::current_exe()?)?,
        input_case_file_sha256: hash_file(cases_path)?,
        seed: 20261109,
        repeat_penalty: if arm == "decoding" { 1.1 } else { 1. },
        server: cfg,
        protocol: "wsp-synthetic-v1-temp0.6-topk20-topp0.95-minp0-presence1.5-full-history-32gram4"
            .into(),
    };
    fs::create_dir(out).context("run output directory must be fresh")?;
    write_json_new(&out.join("cases.json"), &cases)?;
    write_json_new(&out.join("settings.json"), &settings)?;
    let mut results = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out.join("results.jsonl"))?;
    let mut attempts = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out.join("attempts.jsonl"))?;
    let server = match OwnedServer::start(model, &out.join("server.log"), &settings.server) {
        Ok(s) => s,
        Err(e) => {
            write_json_new(
                &out.join("launch-error.json"),
                &json!({"error":format!("{e:#}")}),
            )?;
            return Err(e);
        }
    };
    let client = local_client(900)?;
    let mut outcomes = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let seed = settings.seed + i as u32;
        let start = Instant::now();
        let mut item = Outcome {
            id: case.id.clone(),
            seed,
            correct: false,
            thinking_closed: false,
            cap_hit: false,
            repetition: false,
            tokens: 0,
            seconds: 0.,
            error: None,
            response: None,
        };
        let result = (|| -> Result<Value> {
            let template = json!({"messages":[{"role":"user","content":case.prompt}],"add_generation_prompt":true,"chat_template_kwargs":{"enable_thinking":thinking}});
            let rendered: Value = read_json_response(
                client
                    .post(format!("{}/apply-template", server.url))
                    .json(&template)
                    .send()?,
            )?;
            let prompt = rendered["prompt"]
                .as_str()
                .context("template response lacks prompt")?;
            let tail = prompt
                .rsplit_once("<|im_start|>assistant")
                .context("unsupported chat template")?
                .1;
            ensure!(
                if thinking {
                    tail.contains("<think>") && !tail.contains("</think>")
                } else {
                    tail.contains("</think>")
                },
                "template did not honor thinking mode"
            );
            let request = json!({"prompt":prompt,"n_predict":max_tokens,"seed":seed,"temperature":0.6,"top_p":0.95,"top_k":20,"min_p":0.0,"presence_penalty":1.5,"frequency_penalty":0.0,"repeat_penalty":settings.repeat_penalty,"repeat_last_n":settings.server.context,"cache_prompt":false,"return_tokens":true,"stream":false});
            serde_json::to_writer(
                &mut attempts,
                &json!({"id":case.id,"template":template,"rendered":rendered,"request":request}),
            )?;
            attempts.write_all(b"\n")?;
            attempts.flush()?;
            attempts.sync_data()?;
            let response = client
                .post(format!("{}/completion", server.url))
                .json(&request)
                .send()?;
            read_json_response(response)
        })();
        item.seconds = start.elapsed().as_secs_f64();
        match result {
            Ok(raw) => {
                let parsed = (|| -> Result<()> {
                    let text = raw["content"]
                        .as_str()
                        .context("missing completion content")?;
                    let stop = raw["stop_type"].as_str().context("missing stop reason")?;
                    ensure!(
                        ["eos", "word", "limit"].contains(&stop),
                        "unexpected stop reason"
                    );
                    ensure!(
                        raw["truncated"].as_bool() == Some(false),
                        "runtime context truncated"
                    );
                    let gs = &raw["generation_settings"];
                    for (key, expected) in [
                        ("temperature", 0.6),
                        ("top_p", 0.95),
                        ("top_k", 20.),
                        ("min_p", 0.),
                        ("presence_penalty", 1.5),
                        ("repeat_penalty", settings.repeat_penalty),
                        ("n_predict", max_tokens as f64),
                        ("repeat_last_n", settings.server.context as f64),
                        ("seed", seed as f64),
                    ] {
                        ensure!(
                            gs[key]
                                .as_f64()
                                .is_some_and(|v| (v - expected).abs() < 1e-5),
                            "runtime did not confirm {key}"
                        );
                    }
                    let tokens: Vec<u32> = serde_json::from_value(raw["tokens"].clone())
                        .context("missing generated token IDs")?;
                    item.tokens = raw["tokens_predicted"]
                        .as_u64()
                        .context("missing predicted-token count")?
                        as usize;
                    ensure!(
                        tokens.len() <= item.tokens && item.tokens - tokens.len() <= 1,
                        "token count mismatch"
                    );
                    item.cap_hit = stop == "limit";
                    item.thinking_closed = text.contains("</think>");
                    item.repetition = repeated_ngram(&tokens);
                    item.correct = grade(case, text, thinking);
                    Ok(())
                })();
                if let Err(e) = parsed {
                    item.error = Some(format!("{e:#}"));
                }
                item.response = Some(raw);
            }
            Err(e) => item.error = Some(format!("{e:#}")),
        }
        serde_json::to_writer(&mut results, &item)?;
        results.write_all(b"\n")?;
        results.flush()?;
        results.sync_data()?;
        eprintln!(
            "{} {}/{} correct={} tokens={} cap={} error={}",
            arm,
            i + 1,
            cases.len(),
            item.correct,
            item.tokens,
            item.cap_hit,
            item.error.as_deref().unwrap_or("none")
        );
        outcomes.push(item);
    }
    drop(server);
    ensure!(
        hash_file(model)? == settings.model_sha256,
        "model changed during evaluation"
    );
    let summary = summarize(&outcomes);
    write_json_new(&out.join("summary.json"), &summary)?;
    Ok(summary)
}

fn summarize(items: &[Outcome]) -> Value {
    json!({"attempted":items.len(),"correct":items.iter().filter(|x|x.correct && x.error.is_none()).count(),"errors":items.iter().filter(|x|x.error.is_some()).count(),"cap_hits":items.iter().filter(|x|x.cap_hit).count(),"thinking_closed":items.iter().filter(|x|x.thinking_closed).count(),"repetition_flags":items.iter().filter(|x|x.repetition).count(),"tokens":items.iter().map(|x|x.tokens).sum::<usize>(),"seconds":items.iter().map(|x|x.seconds).sum::<f64>()})
}

pub fn compare(left: &Path, right: &Path) -> Result<Value> {
    let read_settings = |p: &Path| -> Result<Settings> {
        Ok(serde_json::from_reader(File::open(
            p.join("settings.json"),
        )?)?)
    };
    let a = read_settings(left)?;
    let b = read_settings(right)?;
    for s in [&a, &b] {
        ensure!(
            ["base", "edit", "decoding"].contains(&s.arm.as_str()),
            "primary comparisons require base/edit/decoding arms"
        );
        let expected = if s.arm == "decoding" { 1.1 } else { 1.0 };
        ensure!(
            (s.repeat_penalty - expected).abs() < 1e-9,
            "undeclared decoding penalty"
        );
    }
    if (a.arm == "edit") == (b.arm == "edit") {
        ensure!(a.model_sha256 == b.model_sha256, "control weights differ");
    } else {
        ensure!(
            a.model_sha256 != b.model_sha256,
            "edited and base checkpoints have identical hashes"
        );
    }
    ensure!(
        a.case_sha256 == b.case_sha256
            && a.tool_sha256 == b.tool_sha256
            && a.input_case_file_sha256 == b.input_case_file_sha256
            && a.max_tokens == b.max_tokens
            && a.thinking == b.thinking
            && a.seed == b.seed
            && a.protocol == b.protocol,
        "mismatched cases/generation protocol"
    );
    ensure!(
        a.server.image == b.server.image
            && a.server.gpu == b.server.gpu
            && a.server.context == b.server.context,
        "mismatched runtime"
    );
    for p in [left, right] {
        let cases: Vec<Case> = serde_json::from_reader(File::open(p.join("cases.json"))?)?;
        ensure!(
            hash_bytes(&serde_json::to_vec(&cases)?) == a.case_sha256,
            "case file hash mismatch"
        );
    }
    let read = |p: &Path| -> Result<Vec<Outcome>> {
        BufReader::new(File::open(p.join("results.jsonl"))?)
            .lines()
            .map(|l| Ok(serde_json::from_str(&l?)?))
            .collect()
    };
    let ar = read(left)?;
    let br = read(right)?;
    let cases: Vec<Case> = serde_json::from_reader(File::open(left.join("cases.json"))?)?;
    ensure!(
        ar.len() == cases.len() && br.len() == cases.len() && !ar.is_empty(),
        "missing/extra outcome rows"
    );
    let mut differences = Vec::new();
    let (mut wins, mut losses) = (0usize, 0usize);
    for (i, ((x, y), case)) in ar.iter().zip(&br).zip(cases).enumerate() {
        ensure!(
            x.id == y.id && x.id == case.id && x.seed == y.seed && x.seed == a.seed + i as u32,
            "mismatched case identity/order"
        );
        let (x, y) = (
            x.correct && x.error.is_none(),
            y.correct && y.error.is_none(),
        );
        let d = i32::from(y) - i32::from(x);
        if d > 0 {
            wins += 1;
        }
        if d < 0 {
            losses += 1;
        }
        differences.push(f64::from(d));
    }
    let n = differences.len();
    let difference = differences.iter().sum::<f64>() / n as f64;
    let mut rng = Rng(0x731945abc);
    let mut samples = Vec::with_capacity(10000);
    for _ in 0..10000 {
        samples.push(
            (0..n)
                .map(|_| differences[rng.next() as usize % n])
                .sum::<f64>()
                / n as f64,
        );
    }
    samples.sort_by(f64::total_cmp);
    let discordant = wins + losses;
    let mut term = 2f64.powi(-(discordant as i32));
    let mut tail = term;
    for k in 1..=wins.min(losses) {
        term *= ((discordant - k + 1) as f64) / (k as f64);
        tail += term;
    }
    Ok(
        json!({"left":a.arm,"right":b.arm,"n":n,"left_summary":summarize(&ar),"right_summary":summarize(&br),"rescued":wins,"regressed":losses,"paired_accuracy_difference":difference,"paired_bootstrap_95_percentile":[samples[249],samples[9749]],"mcnemar_exact_two_sided_p":(2.*tail).min(1.),"caveat":"Small synthetic suite; errors remain incorrect outcomes; no benchmark or broad capability claim."}),
    )
}
