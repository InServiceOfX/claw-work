//! The brand recipe: colors, fonts, sizes and pacing. It can live inside a spec
//! (`"style": {...}`) or in a standalone `style.json` passed with `--style`, so
//! every Short from one project looks the same.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Style {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub muted: String,
    pub panel: String,
    pub pass: String,
    pub warn: String,
    pub fail: String,
    pub font: String,
    pub mono_font: String,
    /// Silence added after each scene's narration, in milliseconds.
    pub pad_ms: u32,
    pub title_size: u32,
    pub body_size: u32,
}

/// Default fonts, best first. Liberation is metric-compatible with Arial and
/// Courier New, so text measured on macOS keeps its size on Linux.
const FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
];
const MONO_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Courier New Bold.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
];

/// The first candidate that exists, or the first one so the error names a path.
fn first_font(candidates: &[&str]) -> String {
    candidates
        .iter()
        .find(|p| std::path::Path::new(p).is_file())
        .unwrap_or(&candidates[0])
        .to_string()
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: 1080,
            height: 1920,
            fps: 30,
            bg: "#0b0f14".into(),
            fg: "#ffffff".into(),
            accent: "#4ade80".into(),
            muted: "#9aa4b2".into(),
            panel: "#18202b".into(),
            pass: "#4ade80".into(),
            warn: "#fbbf24".into(),
            fail: "#f87171".into(),
            font: first_font(FONT_CANDIDATES),
            mono_font: first_font(MONO_FONT_CANDIDATES),
            pad_ms: 350,
            title_size: 96,
            body_size: 54,
        }
    }
}

/// `#rrggbb` to FFmpeg's `0xrrggbb`.
pub fn ff_color(hex: &str) -> Result<String, String> {
    let digits = hex.strip_prefix('#').unwrap_or("");
    if digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(format!("0x{}", digits.to_ascii_lowercase()))
    } else {
        Err(format!("color {hex:?} must look like #rrggbb"))
    }
}

impl Style {
    /// Every problem with the recipe, empty when it is usable.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !(2..=3840).contains(&self.width) || self.width % 2 != 0 {
            out.push(format!("style.width {} must be even and 2..3840", self.width));
        }
        if !(2..=3840).contains(&self.height) || self.height % 2 != 0 {
            out.push(format!("style.height {} must be even and 2..3840", self.height));
        }
        if !(1..=60).contains(&self.fps) {
            out.push(format!("style.fps {} must be 1..60", self.fps));
        }
        if self.pad_ms > 5000 {
            out.push("style.pad_ms must be at most 5000".into());
        }
        if !(24..=200).contains(&self.title_size) || !(24..=120).contains(&self.body_size) {
            out.push("style.title_size must be 24..200 and body_size 24..120".into());
        }
        for (name, value) in [
            ("bg", &self.bg),
            ("fg", &self.fg),
            ("accent", &self.accent),
            ("muted", &self.muted),
            ("panel", &self.panel),
            ("pass", &self.pass),
            ("warn", &self.warn),
            ("fail", &self.fail),
        ] {
            if let Err(e) = ff_color(value) {
                out.push(format!("style.{name}: {e}"));
            }
        }
        for (name, value) in [("font", &self.font), ("mono_font", &self.mono_font)] {
            if value.is_empty() || value.contains(['\'', '\n', ':']) {
                out.push(format!("style.{name} must be a path without quotes or colons"));
            }
        }
        out
    }

    /// Fonts must exist at render time (not at check time, so specs stay portable).
    pub fn missing_fonts(&self) -> Vec<String> {
        [&self.font, &self.mono_font]
            .into_iter()
            .filter(|p| !std::path::Path::new(p.as_str()).is_file())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fonts_fall_back_in_order() {
        let dir = std::env::temp_dir().join(format!("claw-shorts-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let present = dir.join("present.ttf");
        std::fs::write(&present, b"x").unwrap();
        let present = present.to_str().unwrap();
        assert_eq!(first_font(&["/no/such/font.ttf", present]), present);
        assert_eq!(first_font(&["/no/such/a.ttf", "/no/such/b.ttf"]), "/no/such/a.ttf");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn default_style_is_valid() {
        assert!(Style::default().problems().is_empty());
    }

    #[test]
    fn colors_convert_and_reject() {
        assert_eq!(ff_color("#0B0F14").unwrap(), "0x0b0f14");
        assert!(ff_color("0b0f14").is_err());
        assert!(ff_color("#12345").is_err());
        assert!(ff_color("#12345g").is_err());
    }

    #[test]
    fn bad_values_are_reported() {
        let style = Style { width: 1081, fps: 0, accent: "red".into(), ..Style::default() };
        let problems = style.problems().join("\n");
        assert!(problems.contains("width"), "{problems}");
        assert!(problems.contains("fps"), "{problems}");
        assert!(problems.contains("accent"), "{problems}");
    }

    #[test]
    fn unknown_style_fields_are_errors() {
        let parsed: Result<Style, _> = serde_json::from_str(r#"{"sparkle": true}"#);
        assert!(parsed.is_err());
    }
}
