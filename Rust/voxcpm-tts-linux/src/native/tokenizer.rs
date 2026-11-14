//! Text → token ids exactly as VoxCPM2 expects (PORTING_SPEC §3).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, anyhow};

pub const AUDIO_START: u32 = 101;
pub const AUDIO_END: u32 = 102;
pub const REF_AUDIO_START: u32 = 103;
pub const REF_AUDIO_END: u32 = 104;
const UNK_ID: u32 = 0;

pub struct TextTokenizer {
    inner: tokenizers::Tokenizer,
    vocab: HashMap<String, u32>,
    /// Vocabulary entries of ≥2 characters, all CJK unified ideographs.
    multichar_cjk: HashSet<String>,
}

fn is_cjk(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

impl TextTokenizer {
    pub fn load(tokenizer_json: &Path) -> Result<Self> {
        let inner = tokenizers::Tokenizer::from_file(tokenizer_json)
            .map_err(|e| anyhow!("loading {}: {e}", tokenizer_json.display()))?;
        let vocab = inner.get_vocab(true);
        let multichar_cjk = vocab
            .keys()
            .filter(|t| t.chars().count() >= 2 && t.chars().all(is_cjk))
            .cloned()
            .collect();
        Ok(Self {
            inner,
            vocab,
            multichar_cjk,
        })
    }

    /// Whitespace pre-normalization applied to all text the model sees:
    /// newlines become spaces and whitespace runs collapse to one space.
    pub fn prenormalize(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut in_space = false;
        for c in text.chars() {
            if c.is_whitespace() {
                if !in_space {
                    out.push(' ');
                }
                in_space = true;
            } else {
                out.push(c);
                in_space = false;
            }
        }
        out
    }

    /// Token strings after the leading-space rule and the CJK split.
    pub fn tokens(&self, text: &str) -> Result<Vec<String>> {
        // transformers' legacy Llama path drops exactly one leading space.
        let text = text.strip_prefix(' ').unwrap_or(text);
        let enc = self
            .inner
            .encode(text, false)
            .map_err(|e| anyhow!("tokenizing: {e}"))?;
        let mut out = Vec::with_capacity(enc.get_tokens().len());
        for tok in enc.get_tokens() {
            let clean = tok.replace('▁', "");
            if self.multichar_cjk.contains(&clean) {
                out.extend(clean.chars().map(|c| c.to_string()));
            } else {
                out.push(tok.clone());
            }
        }
        Ok(out)
    }

    /// Token ids (no BOS/EOS; unknown tokens map to 0).
    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        Ok(self
            .tokens(text)?
            .iter()
            .map(|t| self.vocab.get(t).copied().unwrap_or(UNK_ID))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prenormalize_collapses_whitespace_without_trimming() {
        assert_eq!(TextTokenizer::prenormalize("a\n\nb  \tc "), "a b c ");
    }

    #[test]
    fn t1_matches_golden_tokenizer_cases() {
        let Some((golden, snapshot)) = crate::native::golden::require() else {
            return;
        };
        let tok = TextTokenizer::load(&snapshot.join("tokenizer.json")).unwrap();
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(golden.join("tokens.json")).unwrap())
                .unwrap();
        assert!(!cases.is_empty());
        for case in &cases {
            let text = case["text"].as_str().unwrap();
            let want: Vec<u32> = case["ids_final"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            assert_eq!(tok.encode(text).unwrap(), want, "case {text:?}");
        }
        let meta = crate::native::golden::meta().unwrap();
        let ids: Vec<u32> = meta["text_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let text = TextTokenizer::prenormalize(meta["text"].as_str().unwrap());
        assert_eq!(tok.encode(&text).unwrap(), ids);
    }
}
