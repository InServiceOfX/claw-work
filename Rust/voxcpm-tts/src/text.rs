//! Text preparation: normalization, sentence segmentation and chunk packing.
//!
//! VoxCPM2 is most stable on short-to-medium inputs; long scripts are split at
//! sentence boundaries into chunks of at most `max_chars` characters. A chunk
//! never ends mid-sentence unless a single sentence is itself too long, in which
//! case it is split at clause punctuation, then whitespace, then (for scripts
//! without spaces, e.g. CJK) at a character boundary.

/// The pause the stitcher inserts after a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pause {
    /// The chunk ends a sentence inside a paragraph.
    Sentence,
    /// The chunk ends a paragraph.
    Paragraph,
    /// The chunk ends inside an over-long sentence (clause or word split).
    Clause,
    /// Last chunk of the script.
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    /// 1-based position in the script.
    pub index: usize,
    pub text: String,
    pub pause_after: Pause,
}

impl Chunk {
    /// Number of Unicode scalar values; the unit used for sizing and ETA.
    pub fn char_len(&self) -> usize {
        self.text.chars().count()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkOptions {
    /// Upper bound on characters per chunk.
    pub max_chars: usize,
    /// Paragraphs shorter than this are merged with the following paragraph so
    /// the model is not asked to speak a two-word fragment in isolation.
    pub min_chars: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            max_chars: crate::config::DEFAULT_MAX_CHARS,
            min_chars: 40,
        }
    }
}

/// Normalize line endings, strip a BOM and control characters, collapse runs of
/// horizontal whitespace, and keep paragraph breaks (blank lines) as `\n\n`.
pub fn normalize(input: &str) -> String {
    let text = input
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let cleaned: String = line
            .chars()
            .map(|c| {
                if c == '\t' || (c.is_control() && c != '\n') {
                    ' '
                } else {
                    c
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if cleaned.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(cleaned);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs.join("\n\n")
}

const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "u.s", "u.k",
    "a.m", "p.m", "no", "fig", "inc", "ltd", "co", "corp", "dept", "est", "approx", "mt", "jan",
    "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec",
];

fn is_terminal(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…' | '。' | '！' | '？' | '；')
}

fn is_cjk_terminal(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '；')
}

fn is_closer(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | ')' | ']' | '}' | '”' | '’' | '」' | '』' | '）' | '》'
    )
}

/// Whether a `.` at byte offset `dot` ends an abbreviation or an initial.
fn ends_abbreviation(sentence_so_far: &str) -> bool {
    let word: String = sentence_so_far
        .trim_end_matches('.')
        .rsplit(|c: char| c.is_whitespace() || c == '(' || c == '"')
        .next()
        .unwrap_or("")
        .to_lowercase();
    if word.is_empty() {
        return false;
    }
    // Single-letter initials such as "J. R. R. Tolkien".
    if word.chars().count() == 1 && word.chars().all(char::is_alphabetic) {
        return true;
    }
    ABBREVIATIONS.contains(&word.as_str())
}

/// Split one paragraph into sentences, keeping terminal punctuation and
/// closing quotes/brackets attached to the sentence they end.
pub fn split_sentences(paragraph: &str) -> Vec<String> {
    let chars: Vec<char> = paragraph.chars().collect();
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        current.push(c);
        if is_terminal(c) {
            // Absorb runs like "?!", "..." and closing quotes.
            while i + 1 < chars.len() && (is_terminal(chars[i + 1]) || is_closer(chars[i + 1])) {
                i += 1;
                current.push(chars[i]);
            }
            let at_end = i + 1 >= chars.len();
            let next_is_space = !at_end && chars[i + 1].is_whitespace();
            // A '.' not followed by whitespace ("3.14", "example.com") or ending an
            // abbreviation/initial does not end the sentence.
            let boundary = is_cjk_terminal(c)
                || ((at_end || next_is_space) && !(c == '.' && ends_abbreviation(&current)));
            if boundary {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    sentences.push(s);
                }
                current.clear();
            }
        }
        i += 1;
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        sentences.push(s);
    }
    sentences
}

/// Split a sentence longer than `max_chars` into pieces of at most `max_chars`,
/// preferring clause punctuation, then whitespace, then a hard character cut.
fn split_long_sentence(sentence: &str, max_chars: usize) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut rest: Vec<char> = sentence.chars().collect();
    while rest.len() > max_chars {
        let window = &rest[..max_chars];
        let clause = window
            .iter()
            .rposition(|&c| matches!(c, ',' | ';' | ':' | '—' | '–' | '，' | '、' | '：'))
            .filter(|&p| p + 1 >= max_chars / 3);
        let cut = match clause {
            Some(p) => p + 1,
            None => match window
                .iter()
                .rposition(|c| c.is_whitespace())
                .filter(|&p| p > 0)
            {
                Some(p) => p,
                None => max_chars,
            },
        };
        let piece: String = rest[..cut].iter().collect();
        let piece = piece.trim().to_string();
        if !piece.is_empty() {
            pieces.push(piece);
        }
        rest = rest[cut..].to_vec();
        while rest.first().is_some_and(|c| c.is_whitespace()) {
            rest.remove(0);
        }
    }
    let tail: String = rest.into_iter().collect();
    if !tail.trim().is_empty() {
        pieces.push(tail.trim().to_string());
    }
    pieces
}

/// Normalize `input` and pack it into chunks for synthesis.
pub fn chunk_text(input: &str, opts: ChunkOptions) -> Vec<Chunk> {
    let max_chars = opts.max_chars.max(20);
    let normalized = normalize(input);
    // Units: (text, pause after the unit).
    let mut units: Vec<(String, Pause)> = Vec::new();
    let paragraphs: Vec<&str> = normalized
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();
    for paragraph in &paragraphs {
        let sentences = split_sentences(paragraph);
        let n = sentences.len();
        for (si, sentence) in sentences.into_iter().enumerate() {
            let last_in_paragraph = si + 1 == n;
            if sentence.chars().count() > max_chars {
                let pieces = split_long_sentence(&sentence, max_chars);
                let pn = pieces.len();
                for (pi, piece) in pieces.into_iter().enumerate() {
                    let pause = if pi + 1 < pn {
                        Pause::Clause
                    } else if last_in_paragraph {
                        Pause::Paragraph
                    } else {
                        Pause::Sentence
                    };
                    units.push((piece, pause));
                }
            } else {
                let pause = if last_in_paragraph {
                    Pause::Paragraph
                } else {
                    Pause::Sentence
                };
                units.push((sentence, pause));
            }
        }
    }

    // Greedy packing. A paragraph boundary closes the chunk unless the chunk is
    // still shorter than `min_chars`; a clause split always closes it.
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut text = String::new();
    let mut pause = Pause::Sentence;
    for (unit, unit_pause) in units {
        let sep = if text.is_empty() {
            ""
        } else if pause == Pause::Paragraph {
            "\n"
        } else {
            " "
        };
        let fits = text.chars().count() + sep.chars().count() + unit.chars().count() <= max_chars;
        let must_close = !text.is_empty()
            && (pause == Pause::Clause
                || (pause == Pause::Paragraph && text.chars().count() >= opts.min_chars));
        if !text.is_empty() && (!fits || must_close) {
            chunks.push(Chunk {
                index: chunks.len() + 1,
                text: std::mem::take(&mut text),
                pause_after: pause,
            });
        }
        if !text.is_empty() {
            text.push_str(if pause == Pause::Paragraph { "\n" } else { " " });
        }
        text.push_str(&unit);
        pause = unit_pause;
    }
    if !text.is_empty() {
        chunks.push(Chunk {
            index: chunks.len() + 1,
            text,
            pause_after: Pause::End,
        });
    }
    if let Some(last) = chunks.last_mut() {
        last.pause_after = Pause::End;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(max: usize) -> ChunkOptions {
        ChunkOptions {
            max_chars: max,
            min_chars: 40,
        }
    }

    #[test]
    fn normalize_collapses_whitespace_and_keeps_paragraphs() {
        let s = "\u{feff}Hello   world.\r\nSame\tparagraph.\r\n\r\n\r\nNext one.  ";
        assert_eq!(normalize(s), "Hello world. Same paragraph.\n\nNext one.");
    }

    #[test]
    fn sentences_respect_abbreviations_decimals_and_initials() {
        let s = split_sentences(
            "Dr. Smith paid $3.50 at 5 p.m. today. J. R. R. Tolkien wrote it! Did he?",
        );
        assert_eq!(
            s,
            vec![
                "Dr. Smith paid $3.50 at 5 p.m. today.",
                "J. R. R. Tolkien wrote it!",
                "Did he?"
            ]
        );
    }

    #[test]
    fn sentences_keep_closing_quotes() {
        let s = split_sentences("He said \"stop.\" Then he left...");
        assert_eq!(s, vec!["He said \"stop.\"", "Then he left..."]);
    }

    #[test]
    fn cjk_terminals_split_without_spaces() {
        let s = split_sentences("你好。今天天气很好！我们走吧？");
        assert_eq!(s, vec!["你好。", "今天天气很好！", "我们走吧？"]);
    }

    #[test]
    fn short_text_is_one_chunk() {
        let c = chunk_text("Hello there. General Kenobi.", opts(300));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "Hello there. General Kenobi.");
        assert_eq!(c[0].pause_after, Pause::End);
    }

    #[test]
    fn chunks_never_exceed_max_and_preserve_all_words() {
        let sentence = "The quick brown fox jumps over the lazy dog near the riverbank today. ";
        let text = sentence.repeat(30);
        let chunks = chunk_text(&text, opts(200));
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(c.char_len() <= 200, "chunk too long: {}", c.char_len());
            assert!(
                c.text.ends_with('.'),
                "chunk must end at a sentence: {:?}",
                c.text
            );
        }
        let rejoined: Vec<String> = chunks
            .iter()
            .flat_map(|c| c.text.split_whitespace().map(str::to_string))
            .collect();
        let original: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        assert_eq!(rejoined, original);
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.index, i + 1);
        }
    }

    #[test]
    fn over_long_sentence_splits_at_clauses_then_words() {
        let long = format!(
            "{}, {}",
            "word ".repeat(30).trim(),
            "more ".repeat(30).trim()
        );
        let chunks = chunk_text(&long, opts(100));
        assert!(chunks.len() >= 2);
        for c in &chunks {
            assert!(c.char_len() <= 100);
        }
        assert!(chunks[0].text.ends_with(',') || !chunks[0].text.ends_with(' '));
        assert_eq!(chunks[0].pause_after, Pause::Clause);
    }

    #[test]
    fn unspaced_script_is_hard_split() {
        let text = "长".repeat(250);
        let chunks = chunk_text(&text, opts(100));
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|c| c.char_len() <= 100));
    }

    #[test]
    fn paragraph_boundaries_close_chunks_and_tiny_paragraphs_merge() {
        let text = "Title.\n\nThis paragraph is long enough to stand on its own as a chunk.\n\nSecond paragraph here, also reasonably long for testing.";
        let chunks = chunk_text(text, opts(300));
        assert_eq!(chunks.len(), 2, "{chunks:?}");
        assert!(chunks[0].text.starts_with("Title.\nThis paragraph"));
        assert_eq!(chunks[0].pause_after, Pause::Paragraph);
        assert_eq!(chunks[1].pause_after, Pause::End);
    }

    #[test]
    fn empty_and_whitespace_input_yield_no_chunks() {
        assert!(chunk_text("", opts(100)).is_empty());
        assert!(chunk_text(" \n\n\t ", opts(100)).is_empty());
    }
}
