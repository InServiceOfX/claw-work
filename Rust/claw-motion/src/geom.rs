//! 2D vectors and colors shared by the physics, morph and raster code.

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// A point or vector in pixels (y grows downward, as on screen).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "[f64; 2]", into = "[f64; 2]")]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len_sq(self) -> f64 {
        self.dot(self)
    }
    pub fn len(self) -> f64 {
        self.len_sq().sqrt()
    }
    /// Perpendicular, rotated a quarter turn.
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }
    /// Unit vector, or `fallback` when the length is (nearly) zero.
    pub fn normalize_or(self, fallback: Vec2) -> Vec2 {
        let l = self.len();
        if l > 1e-12 { self * (1.0 / l) } else { fallback }
    }
    pub fn lerp(self, o: Vec2, t: f64) -> Vec2 {
        self + (o - self) * t
    }
}

impl From<[f64; 2]> for Vec2 {
    fn from(a: [f64; 2]) -> Self {
        Vec2::new(a[0], a[1])
    }
}
impl From<Vec2> for [f64; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}
impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}
impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}
impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        *self = *self + o;
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Vec2) {
        *self = *self - o;
    }
}

/// An opaque color written as `#rrggbb` in scene files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn parse(s: &str) -> Result<Rgb, String> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("color {s:?} is not #rrggbb"));
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string());
        Ok(Rgb(byte(0)?, byte(2)?, byte(4)?))
    }

    /// Linear blend in sRGB space; good enough for short color morphs.
    pub fn lerp(self, o: Rgb, t: f64) -> Rgb {
        let mix = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round().clamp(0.0, 255.0) as u8;
        Rgb(mix(self.0, o.0), mix(self.1, o.1), mix(self.2, o.2))
    }

    /// The same hue scaled toward black, for outlines.
    pub fn darken(self, factor: f64) -> Rgb {
        let d = |c: u8| (f64::from(c) * factor).round().clamp(0.0, 255.0) as u8;
        Rgb(d(self.0), d(self.1), d(self.2))
    }
}

impl TryFrom<String> for Rgb {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        Rgb::parse(&s)
    }
}
impl From<Rgb> for String {
    fn from(c: Rgb) -> String {
        format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
    }
}

/// Seeded SplitMix64: the same scene file always spawns the same bodies.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_round_trip_and_reject_bad_input() {
        let c = Rgb::parse("#ff5a4e").unwrap();
        assert_eq!(c, Rgb(255, 90, 78));
        assert_eq!(String::from(c), "#ff5a4e");
        assert!(Rgb::parse("#fff").is_err());
        assert!(Rgb::parse("#gg0000").is_err());
        assert_eq!(Rgb(0, 0, 0).lerp(Rgb(200, 100, 50), 0.5), Rgb(100, 50, 25));
    }

    #[test]
    fn rng_is_seeded_and_in_range() {
        let a: Vec<f64> = { let mut r = Rng::new(7); (0..5).map(|_| r.unit()).collect() };
        let b: Vec<f64> = { let mut r = Rng::new(7); (0..5).map(|_| r.unit()).collect() };
        assert_eq!(a, b);
        let mut r = Rng::new(1);
        assert!((0..1000).map(|_| r.range(-2.0, 3.0)).all(|v| (-2.0..3.0).contains(&v)));
    }

    #[test]
    fn vectors() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.len(), 5.0);
        assert_eq!(v.perp().dot(v), 0.0);
        assert_eq!(Vec2::ZERO.normalize_or(Vec2::new(0.0, 1.0)), Vec2::new(0.0, 1.0));
        let parsed: Vec2 = serde_json::from_str("[1.5, -2]").unwrap();
        assert_eq!(parsed, Vec2::new(1.5, -2.0));
    }
}
