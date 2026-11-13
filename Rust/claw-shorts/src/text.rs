//! Small text-layout helpers. FFmpeg's drawtext cannot wrap or shrink text, so
//! the layout is computed here from conservative per-character width ratios.

/// Average advance width of the bundled proportional bold font, in em.
pub const PROP_EM: f64 = 0.60;
/// Advance width of the bundled monospace font, in em (exact for Courier New).
pub const MONO_EM: f64 = 0.60;

/// Characters the default fonts can draw. Anything else renders as a tofu box.
pub fn glyph_ok(c: char) -> bool {
    matches!(c,
        '\u{20}'..='\u{7e}'
        | '\u{a0}'..='\u{ff}'
        | '\u{2013}' | '\u{2014}' | '\u{2018}' | '\u{2019}'
        | '\u{201c}' | '\u{201d}' | '\u{2026}')
}

/// The first character in `s` that cannot be drawn, if any.
pub fn first_bad_glyph(s: &str) -> Option<char> {
    s.chars().find(|c| !glyph_ok(*c))
}

/// Largest font size (px) in `[min, max]` at which `chars` characters fit in
/// `max_px`. Returns `min` when even that is too wide; callers decide whether
/// that is an error.
pub fn fit_size(max_px: f64, chars: usize, em: f64, max: u32, min: u32) -> u32 {
    if chars == 0 {
        return max;
    }
    let fitted = (max_px / (chars as f64 * em)).floor() as i64;
    fitted.clamp(min as i64, max as i64) as u32
}

/// Greedy word wrap into lines of at most `max_chars` characters. A single
/// word longer than the limit is kept whole on its own line.
pub fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Like [`wrap`] but evens out the lines: it uses the same number of lines as
/// the greedy wrap, at the narrowest width that still fits, so a short last
/// line (a lone word) is avoided.
pub fn wrap_balanced(text: &str, max_chars: usize) -> Vec<String> {
    let greedy = wrap(text, max_chars);
    if greedy.len() < 2 {
        return greedy;
    }
    let longest_word = text.split_whitespace().map(|w| w.chars().count()).max().unwrap_or(1);
    (longest_word..=max_chars.max(longest_word))
        .map(|width| wrap(text, width))
        .find(|lines| lines.len() == greedy.len())
        .unwrap_or(greedy)
}

/// Split a URL into lines of at most `max_chars`, breaking only after a `/`.
pub fn split_url(url: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for segment in url.split_inclusive('/') {
        if !current.is_empty()
            && current.chars().count() + segment.chars().count() > max_chars
        {
            lines.push(std::mem::take(&mut current));
        }
        current.push_str(segment);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Case-insensitive whole-word (or whole-phrase) containment.
pub fn contains_term(haystack: &str, term: &str) -> bool {
    let hay = haystack.to_lowercase();
    let term = term.to_lowercase();
    if term.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(pos) = hay[from..].find(&term) {
        let start = from + pos;
        let end = start + term.len();
        let before_ok = hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = hay[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        from = start + term.chars().next().map_or(1, |c| c.len_utf8());
        if from >= hay.len() {
            break;
        }
    }
    false
}

/// FNV-1a 64-bit, used to name cached audio so a changed script or voice
/// never reuses a stale file.
pub fn fnv64(parts: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for part in parts {
        for byte in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_glyphs_the_font_lacks() {
        assert!(glyph_ok('a') && glyph_ok('\u{d7}') && glyph_ok('\u{b7}'));
        assert_eq!(first_bad_glyph("Wrote \u{2713} ok"), Some('\u{2713}'));
        assert_eq!(first_bad_glyph("plain text"), None);
    }

    #[test]
    fn fit_size_shrinks_and_clamps() {
        assert_eq!(fit_size(900.0, 5, PROP_EM, 90, 40), 90);
        assert_eq!(fit_size(900.0, 25, PROP_EM, 90, 40), 60);
        assert_eq!(fit_size(900.0, 100, PROP_EM, 90, 40), 40);
    }

    #[test]
    fn wrap_is_greedy_and_keeps_long_words() {
        assert_eq!(wrap("aa bb cc dd", 5), vec!["aa bb", "cc dd"]);
        assert_eq!(wrap("supercalifragilistic x", 5), vec!["supercalifragilistic", "x"]);
        assert!(wrap("   ", 10).is_empty());
    }

    #[test]
    fn balanced_wrap_avoids_orphans_without_adding_lines() {
        let text = "Runs locally. $0, no API key.";
        let greedy = wrap(text, 27);
        let balanced = wrap_balanced(text, 27);
        assert_eq!(greedy, vec!["Runs locally. $0, no API", "key."]);
        assert_eq!(balanced.len(), 2);
        assert!(balanced.iter().all(|l| l.split_whitespace().count() > 1), "{balanced:?}");
        assert!(balanced.iter().all(|l| l.chars().count() <= 27));
        assert_eq!(wrap_balanced("short", 27), vec!["short"]);
        assert_eq!(wrap_balanced("Quality vs ElevenLabs. You judge.", 27).len(), 2);
    }

    #[test]
    fn split_url_breaks_only_after_slashes() {
        let lines = split_url("github.com/Org/repo/tree/master/Rust/app", 22);
        assert_eq!(lines.concat(), "github.com/Org/repo/tree/master/Rust/app");
        assert!(lines.iter().all(|l| l.chars().count() <= 22), "{lines:?}");
        assert!(lines[..lines.len() - 1].iter().all(|l| l.ends_with('/')));
    }

    #[test]
    fn contains_term_respects_word_boundaries() {
        assert!(contains_term("The best TTS", "best"));
        assert!(contains_term("Better than X", "better than"));
        assert!(!contains_term("bestow honor", "best"));
        assert!(!contains_term("the beast", "best"));
        assert!(contains_term("#1 pick", "#1"));
    }

    #[test]
    fn fnv_changes_with_any_part() {
        assert_ne!(fnv64(&["a", "b"]), fnv64(&["ab", ""]));
        assert_eq!(fnv64(&["x"]), fnv64(&["x"]));
    }
}
