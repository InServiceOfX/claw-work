//! Anti-aliased drawing onto an RGBA frame with tiny-skia.

use tiny_skia::{Color, FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::geom::{Rgb, Vec2};

pub struct Frame {
    pixmap: Pixmap,
}

fn paint(c: Rgb) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c.0, c.1, c.2, 255));
    p.anti_alias = true;
    p
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Result<Frame, String> {
        Pixmap::new(width, height).map(|pixmap| Frame { pixmap }).ok_or_else(|| format!("cannot allocate a {width}x{height} frame"))
    }

    pub fn clear(&mut self, c: Rgb) {
        self.pixmap.fill(Color::from_rgba8(c.0, c.1, c.2, 255));
    }

    pub fn circle(&mut self, centre: Vec2, radius: f64, fill: Rgb, outline: Option<(Rgb, f64)>) {
        let Some(path) = PathBuilder::from_circle(centre.x as f32, centre.y as f32, radius.max(0.1) as f32) else { return };
        self.pixmap.fill_path(&path, &paint(fill), FillRule::Winding, Transform::identity(), None);
        if let Some((color, width)) = outline {
            let stroke = Stroke { width: width as f32, ..Stroke::default() };
            self.pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
        }
    }

    /// A segment with round ends and total thickness `2 * radius`.
    pub fn capsule(&mut self, a: Vec2, b: Vec2, radius: f64, color: Rgb) {
        if (a - b).len() < 1e-6 {
            self.circle(a, radius, color, None);
            return;
        }
        let mut pb = PathBuilder::new();
        pb.move_to(a.x as f32, a.y as f32);
        pb.line_to(b.x as f32, b.y as f32);
        let Some(path) = pb.finish() else { return };
        let stroke = Stroke { width: (2.0 * radius) as f32, line_cap: LineCap::Round, ..Stroke::default() };
        self.pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
    }

    pub fn polygon(&mut self, points: &[Vec2], color: Rgb) {
        let mut pb = PathBuilder::new();
        let mut it = points.iter();
        let Some(first) = it.next() else { return };
        pb.move_to(first.x as f32, first.y as f32);
        for p in it {
            pb.line_to(p.x as f32, p.y as f32);
        }
        pb.close();
        let Some(path) = pb.finish() else { return };
        self.pixmap.fill_path(&path, &paint(color), FillRule::Winding, Transform::identity(), None);
    }

    /// Raw RGBA bytes. Every pixel is opaque (the frame is cleared to an opaque
    /// color first), so tiny-skia's premultiplied data equals straight RGBA.
    pub fn rgba(&self) -> &[u8] {
        self.pixmap.data()
    }

    pub fn save_png(&self, path: &std::path::Path) -> Result<(), String> {
        self.pixmap.save_png(path).map_err(|e| format!("write {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(f: &Frame, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * 100 + x) * 4) as usize;
        f.rgba()[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn shapes_land_where_they_are_drawn() {
        let mut f = Frame::new(100, 100).unwrap();
        f.clear(Rgb(10, 20, 30));
        f.circle(Vec2::new(25.0, 25.0), 10.0, Rgb(255, 0, 0), None);
        f.capsule(Vec2::new(60.0, 80.0), Vec2::new(90.0, 80.0), 4.0, Rgb(0, 255, 0));
        f.polygon(&[Vec2::new(60.0, 10.0), Vec2::new(90.0, 10.0), Vec2::new(90.0, 40.0)], Rgb(0, 0, 255));
        assert_eq!(pixel(&f, 25, 25), [255, 0, 0, 255]);
        assert_eq!(pixel(&f, 75, 80), [0, 255, 0, 255]);
        assert_eq!(pixel(&f, 85, 15), [0, 0, 255, 255]);
        assert_eq!(pixel(&f, 5, 95), [10, 20, 30, 255]);
        assert!(f.rgba().chunks(4).all(|p| p[3] == 255));
    }
}
