use crate::error::{Error, Result};
use crate::subtitle::Cue;
use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// Visual style for burned-in captions. Colors are RGBA.
#[derive(Debug, Clone)]
pub struct CaptionStyle {
    pub canvas_width: u32,
    pub font_size: f32,
    /// Fraction of canvas width the text block may occupy.
    pub max_width_ratio: f32,
    pub fore_color: Rgba<u8>,
    pub outline_color: Rgba<u8>,
    pub outline_width: i32,
    /// Semi-transparent box behind the text; None disables it.
    pub background: Option<Rgba<u8>>,
}

impl CaptionStyle {
    pub fn for_canvas(canvas_width: u32, font_size: u32) -> Self {
        Self {
            canvas_width,
            font_size: font_size as f32,
            max_width_ratio: 0.9,
            fore_color: Rgba([255, 255, 255, 255]),
            outline_color: Rgba([0, 0, 0, 255]),
            outline_width: 2,
            background: Some(Rgba([0, 0, 0, 110])),
        }
    }
}

pub struct CaptionRenderer {
    font: FontVec,
}

impl CaptionRenderer {
    pub fn from_font_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|e| Error::Caption(format!("cannot read font {}: {e}", path.display())))?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|e| Error::Caption(format!("cannot parse font {}: {e}", path.display())))?;
        Ok(Self { font })
    }

    fn line_width(&self, text: &str, scale: PxScale) -> f32 {
        let scaled = self.font.as_scaled(scale);
        let mut width = 0.0;
        let mut previous = None;
        for ch in text.chars() {
            let glyph = scaled.glyph_id(ch);
            if let Some(prev) = previous {
                width += scaled.kern(prev, glyph);
            }
            width += scaled.h_advance(glyph);
            previous = Some(glyph);
        }
        width
    }

    /// Greedy word wrap by measured pixel width; single words longer than the
    /// limit (and unspaced CJK runs) are broken per character.
    pub fn wrap_text(&self, text: &str, max_width: f32, scale: PxScale) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in text.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if self.line_width(&candidate, scale) <= max_width {
                current = candidate;
                continue;
            }
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            if self.line_width(word, scale) <= max_width {
                current = word.to_string();
            } else {
                for ch in word.chars() {
                    let candidate = format!("{current}{ch}");
                    if self.line_width(&candidate, scale) > max_width && !current.is_empty() {
                        lines.push(std::mem::take(&mut current));
                        current = ch.to_string();
                    } else {
                        current = candidate;
                    }
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    /// Render one caption to a canvas-wide transparent image, text centered.
    pub fn render(&self, text: &str, style: &CaptionStyle) -> RgbaImage {
        let scale = PxScale::from(style.font_size);
        let scaled = self.font.as_scaled(scale);
        let max_text_width = style.canvas_width as f32 * style.max_width_ratio
            - 2.0 * style.font_size * 0.4;
        let lines = self.wrap_text(text, max_text_width.max(style.font_size), scale);

        let line_height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).ceil();
        let pad_x = style.font_size * 0.4;
        let pad_y = style.font_size * 0.35;
        let outline = style.outline_width.max(0) as f32;
        let text_height = line_height * lines.len() as f32;
        let image_height = (text_height + 2.0 * pad_y + 2.0 * outline).ceil() as u32;
        let mut image = RgbaImage::new(style.canvas_width, image_height.max(1));

        if let Some(bg) = style.background {
            let widest = lines
                .iter()
                .map(|line| self.line_width(line, scale))
                .fold(0.0_f32, f32::max);
            let box_width = (widest + 2.0 * pad_x).min(style.canvas_width as f32);
            let box_x0 = (style.canvas_width as f32 - box_width) / 2.0;
            let radius = (style.font_size * 0.35).min(box_width / 2.0);
            draw_rounded_rect(
                &mut image,
                box_x0,
                0.0,
                box_width,
                image_height as f32,
                radius,
                bg,
            );
        }

        for (index, line) in lines.iter().enumerate() {
            let line_width = self.line_width(line, scale);
            let origin_x = (style.canvas_width as f32 - line_width) / 2.0;
            let baseline_y = pad_y + outline + scaled.ascent() + index as f32 * line_height;
            // Outline first: redraw the line offset in a ring, then the fill.
            let w = style.outline_width;
            if w > 0 {
                for dx in -w..=w {
                    for dy in -w..=w {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        self.draw_line(
                            &mut image,
                            line,
                            origin_x + dx as f32,
                            baseline_y + dy as f32,
                            scale,
                            style.outline_color,
                        );
                    }
                }
            }
            self.draw_line(&mut image, line, origin_x, baseline_y, scale, style.fore_color);
        }
        image
    }

    fn draw_line(
        &self,
        image: &mut RgbaImage,
        text: &str,
        origin_x: f32,
        baseline_y: f32,
        scale: PxScale,
        color: Rgba<u8>,
    ) {
        let scaled = self.font.as_scaled(scale);
        let mut caret = origin_x;
        let mut previous = None;
        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = previous {
                caret += scaled.kern(prev, glyph_id);
            }
            let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(caret, baseline_y));
            if let Some(outlined) = self.font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, coverage| {
                    let px = bounds.min.x as i64 + x as i64;
                    let py = bounds.min.y as i64 + y as i64;
                    blend_pixel(image, px, py, color, coverage);
                });
            }
            caret += scaled.h_advance(glyph_id);
            previous = Some(glyph_id);
        }
    }
}

fn blend_pixel(image: &mut RgbaImage, x: i64, y: i64, color: Rgba<u8>, coverage: f32) {
    if x < 0 || y < 0 || x >= image.width() as i64 || y >= image.height() as i64 {
        return;
    }
    let coverage = coverage.clamp(0.0, 1.0);
    if coverage <= 0.0 {
        return;
    }
    let source_alpha = color.0[3] as f32 / 255.0 * coverage;
    let pixel = image.get_pixel_mut(x as u32, y as u32);
    let dest_alpha = pixel.0[3] as f32 / 255.0;
    let out_alpha = source_alpha + dest_alpha * (1.0 - source_alpha);
    if out_alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let src = color.0[channel] as f32;
        let dst = pixel.0[channel] as f32;
        pixel.0[channel] =
            ((src * source_alpha + dst * dest_alpha * (1.0 - source_alpha)) / out_alpha) as u8;
    }
    pixel.0[3] = (out_alpha * 255.0) as u8;
}

fn draw_rounded_rect(
    image: &mut RgbaImage,
    x0: f32,
    y0: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Rgba<u8>,
) {
    let x1 = x0 + width;
    let y1 = y0 + height;
    for y in y0.max(0.0) as u32..(y1.min(image.height() as f32)) as u32 {
        for x in x0.max(0.0) as u32..(x1.min(image.width() as f32)) as u32 {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            // Distance test against the four corner circles.
            let inside = if fx < x0 + radius && fy < y0 + radius {
                hypot(fx - (x0 + radius), fy - (y0 + radius)) <= radius
            } else if fx > x1 - radius && fy < y0 + radius {
                hypot(fx - (x1 - radius), fy - (y0 + radius)) <= radius
            } else if fx < x0 + radius && fy > y1 - radius {
                hypot(fx - (x0 + radius), fy - (y1 - radius)) <= radius
            } else if fx > x1 - radius && fy > y1 - radius {
                hypot(fx - (x1 - radius), fy - (y1 - radius)) <= radius
            } else {
                true
            };
            if inside {
                blend_pixel(image, x as i64, y as i64, color, 1.0);
            }
        }
    }
}

fn hypot(a: f32, b: f32) -> f32 {
    (a * a + b * b).sqrt()
}

/// Rendered caption overlay ready for ffmpeg composition.
pub struct CaptionOverlay {
    pub png_path: PathBuf,
    pub start: f64,
    pub end: f64,
    pub height: u32,
}

/// Render every cue into `dir` as caption-NNN.png.
pub fn render_overlays(
    cues: &[Cue],
    font_path: &Path,
    style: &CaptionStyle,
    dir: &Path,
) -> Result<Vec<CaptionOverlay>> {
    std::fs::create_dir_all(dir)?;
    let renderer = CaptionRenderer::from_font_file(font_path)?;
    let mut overlays = Vec::new();
    for (index, cue) in cues.iter().enumerate() {
        let image = renderer.render(&cue.text, style);
        let png_path = dir.join(format!("caption-{index:03}.png"));
        image
            .save(&png_path)
            .map_err(|e| Error::Caption(format!("cannot write {}: {e}", png_path.display())))?;
        overlays.push(CaptionOverlay {
            png_path,
            start: cue.start,
            end: cue.end,
            height: image.height(),
        });
    }
    Ok(overlays)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font() -> Option<CaptionRenderer> {
        for candidate in [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ] {
            let path = Path::new(candidate);
            if path.is_file() {
                return CaptionRenderer::from_font_file(path).ok();
            }
        }
        None
    }

    #[test]
    fn wraps_long_text_into_multiple_lines() {
        let Some(renderer) = test_font() else {
            eprintln!("skipping: no system font available");
            return;
        };
        let scale = PxScale::from(64.0);
        let lines = renderer.wrap_text(
            "a fairly long caption that certainly cannot fit on one narrow line",
            400.0,
            scale,
        );
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(renderer.line_width(line, scale) <= 400.0 + 64.0);
        }
    }

    #[test]
    fn renders_nonempty_image() {
        let Some(renderer) = test_font() else {
            eprintln!("skipping: no system font available");
            return;
        };
        let style = CaptionStyle::for_canvas(1080, 64);
        let image = renderer.render("Hello shorts", &style);
        assert_eq!(image.width(), 1080);
        assert!(image.height() > 64);
        let any_opaque = image.pixels().any(|pixel| pixel.0[3] > 200);
        assert!(any_opaque, "expected some visible text pixels");
    }
}
