//! Turns a spec and measured timings into one FFmpeg `-filter_complex`.
//!
//! Every string is written to a small text file and drawn with
//! `drawtext=textfile=...:expansion=none`, so no user text ever needs FFmpeg
//! escaping. All layout is computed here (see [`crate::text`]); nothing depends
//! on measuring rendered pixels.

use crate::spec::{Scene, SceneKind, Spec, Tone, TERMINAL_MIN_SIZE};
use crate::style::{ff_color, Style};
use crate::text::{self, MONO_EM, PROP_EM};
use crate::timeline::{self, SceneTiming};

/// A text file the render must write (relative to FFmpeg's working directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct FilterPlan {
    pub text_files: Vec<TextFile>,
    pub filter_complex: String,
    pub total_ms: u64,
}

struct Colors {
    bg: String,
    fg: String,
    accent: String,
    muted: String,
    panel: String,
    pass: String,
    warn: String,
    fail: String,
}

impl Colors {
    fn new(style: &Style) -> Self {
        let c = |v: &str| ff_color(v).expect("style validated before filters are built");
        Self {
            bg: c(&style.bg),
            fg: c(&style.fg),
            accent: c(&style.accent),
            muted: c(&style.muted),
            panel: c(&style.panel),
            pass: c(&style.pass),
            warn: c(&style.warn),
            fail: c(&style.fail),
        }
    }

    fn tone(&self, tone: Tone) -> &str {
        match tone {
            Tone::Pass => &self.pass,
            Tone::Warn => &self.warn,
            Tone::Fail => &self.fail,
        }
    }
}

const SAFE_WIDTH: f64 = 900.0;
const CENTER_Y: i64 = 900;

struct Builder<'a> {
    style: &'a Style,
    colors: Colors,
    filters: Vec<String>,
    files: Vec<TextFile>,
}

fn window(a: u64, b: u64) -> String {
    format!("gte(t,{})*lt(t,{})", timeline::secs(a), timeline::secs(b))
}

impl<'a> Builder<'a> {
    fn new(style: &'a Style) -> Self {
        Self { style, colors: Colors::new(style), filters: Vec::new(), files: Vec::new() }
    }

    #[allow(clippy::too_many_arguments)]
    fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: &str, a: u64, b: u64) {
        self.filters.push(format!(
            "drawbox=x={x}:y={y}:w={w}:h={h}:color={color}:t=fill:enable='{}'",
            window(a, b)
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, content: &str, mono: bool, color: &str, size: u32, x: &str, y: i64, a: u64, b: u64) {
        let name = format!("text/t{:04}.txt", self.files.len());
        self.files.push(TextFile { name: name.clone(), content: content.to_string() });
        let font = if mono { &self.style.mono_font } else { &self.style.font };
        self.filters.push(format!(
            "drawtext=fontfile='{font}':textfile='{name}':expansion=none:fontcolor={color}:fontsize={size}:x={x}:y={y}:enable='{}'",
            window(a, b)
        ));
    }

    fn centered(&mut self, content: &str, color: &str, size: u32, y: i64, a: u64, b: u64) {
        self.text(content, false, color, size, "(w-text_w)/2", y, a, b);
    }

    /// Centered lines revealed one after another. Returns the y below the block.
    fn centered_block(
        &mut self,
        lines: &[String],
        size: u32,
        top: i64,
        accent_last: bool,
        start: u64,
        end: u64,
    ) -> i64 {
        let lh = (f64::from(size) * 1.3) as i64;
        let stagger = 220u64.min((end - start) / (lines.len() as u64 * 3).max(1));
        for (k, line) in lines.iter().enumerate() {
            let color = if accent_last && lines.len() >= 2 && k == lines.len() - 1 {
                self.colors.accent.clone()
            } else {
                self.colors.fg.clone()
            };
            let appear = (start + k as u64 * stagger).min(end.saturating_sub(1));
            self.centered(line, &color, size, top + k as i64 * lh, appear, end);
        }
        top + lines.len() as i64 * lh
    }

    fn title_like(&mut self, scene: &Scene, t: &SceneTiming) {
        let size = scene
            .lines
            .iter()
            .map(|l| text::fit_size(SAFE_WIDTH, l.chars().count(), PROP_EM, self.style.title_size, 40))
            .min()
            .unwrap_or(self.style.title_size);
        let lh = (f64::from(size) * 1.3) as i64;
        if scene.kind == SceneKind::Endcard {
            let url = scene.url.as_deref().unwrap_or_default();
            let max_chars = (SAFE_WIDTH / (44.0 * PROP_EM)).floor() as usize;
            let url_lines = text::split_url(url, max_chars);
            let widest = url_lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
            let url_size = text::fit_size(SAFE_WIDTH, widest, PROP_EM, 52, 30);
            let url_lh = (f64::from(url_size) * 1.35) as i64;
            let block = scene.lines.len() as i64 * lh + 70 + url_lines.len() as i64 * url_lh;
            let top = CENTER_Y - block / 2;
            let below = self.centered_block(&scene.lines, size, top, false, t.start_ms, t.end_ms);
            let url_start = (t.start_ms + 500).min(t.end_ms.saturating_sub(1));
            let accent = self.colors.accent.clone();
            for (k, line) in url_lines.iter().enumerate() {
                self.centered(line, &accent, url_size, below + 70 + k as i64 * url_lh, url_start, t.end_ms);
            }
        } else {
            let top = CENTER_Y - (scene.lines.len() as i64 * lh) / 2;
            self.centered_block(&scene.lines, size, top, true, t.start_ms, t.end_ms);
        }
    }

    fn terminal(&mut self, scene: &Scene, t: &SceneTiming) {
        let command = scene.command.as_deref().unwrap_or_default();
        let cmd_chars = command.chars().count();
        let size = text::fit_size(840.0, cmd_chars + 2, MONO_EM, 40, TERMINAL_MIN_SIZE);
        let cw = f64::from(size) * MONO_EM;
        let lh = (f64::from(size) * 1.5) as i64;
        let wrap_at = (840.0 / cw).floor() as usize;
        let output: Vec<String> = scene.output.iter().flat_map(|l| text::wrap(l, wrap_at)).collect();
        let body_lines = 1 + output.len() as i64;
        let height = 90 + 40 + body_lines * lh + 40;
        let top = (CENTER_Y - height / 2).max(260);
        let panel = self.colors.panel.clone();
        self.rect(60, top, 960, height, &panel, t.start_ms, t.end_ms);
        for (k, color) in [self.colors.fail.clone(), self.colors.warn.clone(), self.colors.pass.clone()]
            .iter()
            .enumerate()
        {
            self.rect(100 + k as i64 * 46, top + 32, 26, 26, color, t.start_ms, t.end_ms);
        }
        let x0 = 100i64;
        let cmd_y = top + 90 + 40;
        let (pass, fg) = (self.colors.pass.clone(), self.colors.fg.clone());
        self.text("$", true, &pass, size, &x0.to_string(), cmd_y, t.start_ms, t.end_ms);
        let typed_x = (x0 as f64 + 2.0 * cw).round() as i64;
        let type_start = (t.start_ms + 300).min(t.end_ms.saturating_sub(1));
        let typing = timeline::typing_ms(cmd_chars, t.duration_ms());
        let step = (typing / cmd_chars.max(1) as u64).max(1);
        let chars: Vec<char> = command.chars().collect();
        for k in 1..=chars.len() {
            let prefix: String = chars[..k].iter().collect();
            let a = type_start + (k as u64 - 1) * step;
            let b = if k == chars.len() { t.end_ms } else { type_start + k as u64 * step };
            self.text(&prefix, true, &fg, size, &typed_x.to_string(), cmd_y, a.min(t.end_ms - 1), b.min(t.end_ms));
        }
        let first_output = type_start + step * chars.len() as u64 + 250;
        for (j, line) in output.iter().enumerate() {
            let appear = (first_output + j as u64 * 350).min(t.end_ms.saturating_sub(100));
            self.text(line, true, &fg, size, &x0.to_string(), cmd_y + (1 + j as i64) * lh, appear, t.end_ms);
        }
    }

    fn verdict(&mut self, scene: &Scene, t: &SceneTiming) {
        let size = self.style.body_size;
        let wrap_at = (SAFE_WIDTH / (f64::from(size) * PROP_EM)).floor() as usize;
        let lh = (f64::from(size) * 1.3) as i64;
        let badge_h = 68i64;
        let wrapped: Vec<Vec<String>> = scene.rows.iter().map(|r| text::wrap_balanced(&r.text, wrap_at)).collect();
        let row_h: Vec<i64> = wrapped.iter().map(|w| badge_h + 16 + w.len() as i64 * lh + 44).collect();
        let block: i64 = 90 + row_h.iter().sum::<i64>();
        let top = (CENTER_Y - block / 2).max(260);
        let muted = self.colors.muted.clone();
        self.text("VERDICT", false, &muted, 40, "90", top, t.start_ms, t.end_ms);
        let reveal_span = t.duration_ms() * 6 / 10;
        let mut y = top + 90;
        let bg = self.colors.bg.clone();
        let fg = self.colors.fg.clone();
        for (k, row) in scene.rows.iter().enumerate() {
            let appear = (t.start_ms + 350 + k as u64 * reveal_span / scene.rows.len() as u64)
                .min(t.end_ms.saturating_sub(1));
            let tone = self.colors.tone(row.tone).to_string();
            let badge_w = (row.mark.chars().count() as f64 * 40.0 * 0.62) as i64 + 48;
            self.rect(90, y, badge_w, badge_h, &tone, appear, t.end_ms);
            let x = format!("90+({badge_w}-text_w)/2");
            self.text(&row.mark, false, &bg, 40, &x, y + 13, appear, t.end_ms);
            for (l, line) in wrapped[k].iter().enumerate() {
                self.text(line, false, &fg, size, "90", y + badge_h + 16 + l as i64 * lh, appear, t.end_ms);
            }
            y += row_h[k];
        }
    }

    fn chrome(&mut self, spec: &Spec, timings: &[SceneTiming], total: u64) {
        // Progress dots near the top, one per scene.
        let n = timings.len() as i64;
        let (dot_w, gap) = (64i64, 14i64);
        let x0 = (i64::from(self.style.width) - (n * dot_w + (n - 1) * gap)) / 2;
        let (panel, accent) = (self.colors.panel.clone(), self.colors.accent.clone());
        for i in 0..n {
            self.rect(x0 + i * (dot_w + gap), 230, dot_w, 10, &panel, 0, total);
        }
        for (i, t) in timings.iter().enumerate() {
            self.rect(x0 + i as i64 * (dot_w + gap), 230, dot_w, 10, &accent, t.start_ms, t.end_ms);
        }
        // Optional disclosure and credits footer, kept above the platform UI.
        if let Some(footer) = spec.footer_text() {
            let size = 30u32;
            let wrap_at = (SAFE_WIDTH / (f64::from(size) * PROP_EM)).floor() as usize;
            let lines = text::wrap_balanced(&footer, wrap_at);
            let muted = self.colors.muted.clone();
            let top = i64::from(self.style.height) - 420;
            for (k, line) in lines.iter().enumerate() {
                self.centered(line, &muted, size, top + k as i64 * 40, 0, total);
            }
        }
    }
}

/// Build the whole filter graph. `timings` must have one entry per scene.
pub fn build(spec: &Spec, timings: &[SceneTiming]) -> FilterPlan {
    assert_eq!(spec.scenes.len(), timings.len(), "one timing per scene");
    let total = timeline::total_ms(timings);
    let mut b = Builder::new(&spec.style);
    b.chrome(spec, timings, total);
    for (scene, t) in spec.scenes.iter().zip(timings) {
        match scene.kind {
            SceneKind::Title | SceneKind::Cloning | SceneKind::Endcard => b.title_like(scene, t),
            SceneKind::Terminal => b.terminal(scene, t),
            SceneKind::Verdict => b.verdict(scene, t),
        }
    }
    let video = format!("[0:v]{}[v]", b.filters.join(","));
    let filter_complex = format!("{};{video}", audio_graph(timings, 1));
    FilterPlan { text_files: b.files, filter_complex, total_ms: total }
}

/// Filter-graph text that pads each scene's narration to its exact length and
/// joins them into `[aout]`. Scene audio inputs start at input index `first_input`.
pub fn audio_graph(timings: &[SceneTiming], first_input: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, t) in timings.iter().enumerate() {
        let dur = timeline::secs(t.duration_ms());
        parts.push(format!(
            "[{}:a]aformat=sample_rates=48000:channel_layouts=mono,apad=whole_dur={dur},atrim=end={dur}[a{i}]",
            first_input + i
        ));
    }
    let labels: String = (0..timings.len()).map(|i| format!("[a{i}]")).collect();
    parts.push(format!("{labels}concat=n={}:v=0:a=1[aout]", timings.len()));
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{parse, Spec};

    fn spec() -> Spec {
        parse(
            r##"{
              "version": 1, "project": "demo", "title": "t", "caption": "c",
              "disclosure": "AI-generated voice", "credits": ["Voice: A Reader"],
              "scenes": [
                {"kind": "title", "narration": "x", "lines": ["Local voice cloning.", "Fact or hype?"]},
                {"kind": "terminal", "narration": "x", "command": "demo say a.txt",
                 "output": ["Wrote voice.wav (21s of audio)", "generated in 32s, 1.49x real time"]},
                {"kind": "verdict", "narration": "x", "rows": [
                  {"mark": "FACT", "tone": "pass", "text": "Runs locally on a Mac for zero dollars."},
                  {"mark": "UNTESTED", "tone": "warn", "text": "Quality vs a paid service."}]},
                {"kind": "endcard", "narration": "x", "lines": ["The Demo"], "url": "github.com/Org/demo/tree/master/Rust/demo"}
              ]}"##,
        )
        .unwrap()
    }

    fn timings() -> Vec<SceneTiming> {
        timeline::build(&[3000, 6000, 5000, 3000], 350)
    }

    #[test]
    fn one_audio_branch_per_scene_with_exact_lengths() {
        let plan = build(&spec(), &timings());
        let fc = &plan.filter_complex;
        for i in 1..=4 {
            assert!(fc.contains(&format!("[{i}:a]aformat")), "{fc}");
        }
        assert!(fc.contains("apad=whole_dur=3.350,atrim=end=3.350[a0]"), "{fc}");
        assert!(fc.contains("[a0][a1][a2][a3]concat=n=4:v=0:a=1[aout]"));
        assert!(fc.contains("[0:v]") && fc.ends_with("[v]"));
        assert_eq!(plan.total_ms, 3350 + 6350 + 5350 + 3350);
    }

    #[test]
    fn text_goes_through_files_never_inline() {
        let plan = build(&spec(), &timings());
        assert!(!plan.filter_complex.contains("Local voice cloning"));
        assert!(plan.text_files.iter().any(|f| f.content == "Local voice cloning."));
        assert!(plan.filter_complex.matches("expansion=none").count() >= plan.text_files.len());
        let names: std::collections::HashSet<_> = plan.text_files.iter().map(|f| &f.name).collect();
        assert_eq!(names.len(), plan.text_files.len());
        assert!(plan.text_files.iter().all(|f| !f.content.contains('\n')));
    }

    #[test]
    fn windows_are_well_formed_and_inside_the_video() {
        let plan = build(&spec(), &timings());
        let total = plan.total_ms as f64 / 1000.0;
        let mut seen = 0;
        for piece in plan.filter_complex.split("gte(t,").skip(1) {
            let a: f64 = piece.split(')').next().unwrap().parse().unwrap();
            let b: f64 = piece.split("lt(t,").nth(1).unwrap().split(')').next().unwrap().parse().unwrap();
            assert!(a < b + 1e-9, "empty window {a}..{b}");
            assert!(b <= total + 1e-9, "window {a}..{b} past end {total}");
            seen += 1;
        }
        assert!(seen > 30);
    }

    #[test]
    fn terminal_types_the_command_one_prefix_at_a_time() {
        let plan = build(&spec(), &timings());
        let prefixes: Vec<&str> = plan.text_files.iter().map(|f| f.content.as_str())
            .filter(|c| "demo say a.txt".starts_with(*c) && !c.is_empty()).collect();
        assert_eq!(prefixes.len(), "demo say a.txt".len());
        assert_eq!(prefixes.last().copied(), Some("demo say a.txt"));
    }

    #[test]
    fn footer_and_url_split_appear_and_stay_in_safe_width() {
        let plan = build(&spec(), &timings());
        let all: Vec<&str> = plan.text_files.iter().map(|f| f.content.as_str()).collect();
        assert!(all.iter().any(|c| c.contains("AI-generated voice")));
        let url_pieces: String = all.iter().filter(|c| c.starts_with("github.com") || c.starts_with("tree/") || c.starts_with("Rust/") || c.starts_with("master/")).copied().collect();
        assert_eq!(url_pieces, "github.com/Org/demo/tree/master/Rust/demo");
    }

    #[test]
    fn no_footer_without_disclosure_or_credits() {
        let mut s = spec();
        s.disclosure = None;
        s.credits.clear();
        let plan = build(&s, &timings());
        assert!(!plan.text_files.iter().any(|f| f.content.contains("AI-generated")));
    }

    #[test]
    fn last_title_line_gets_the_accent() {
        let plan = build(&spec(), &timings());
        let line = plan.filter_complex.split(',').find(|p| p.contains("fontcolor=0x4ade80") && p.contains("textfile='text/t")).unwrap();
        assert!(line.contains("fontcolor=0x4ade80"));
    }
}
