//! Smooth morphing between two closed shapes.
//!
//! The approach follows Flubber (veltman/flubber, MIT): turn both shapes into
//! rings with the same number of points, give them the same winding, rotate one
//! ring so its points line up with the closest points of the other, then
//! interpolate point by point. This is a fresh implementation: rings are
//! resampled evenly by arc length instead of Flubber's edge bisection, and the
//! rotation search is exhaustive (cheap at a few hundred points).
//!
//! Shapes come from named primitives in a unit box (x and y in -1..1, y down)
//! or from an SVG path's first subpath (M, L, H, V, C, Q, Z, absolute and
//! relative), which is flattened and fitted into the unit box.

use serde::Deserialize;

use crate::geom::Vec2;

/// A closed polygon; the last point connects back to the first.
pub type Ring = Vec<Vec2>;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "shape", rename_all = "lowercase", deny_unknown_fields)]
pub enum ShapeSpec {
    Circle,
    Square,
    Triangle,
    /// Regular polygon with `sides` corners.
    Polygon { sides: usize },
    /// Star with `points` tips; `inner` is the inner radius as a fraction of the outer.
    Star { points: usize, #[serde(default = "default_inner")] inner: f64 },
    Heart,
    /// Explicit corner list in unit-box coordinates.
    Points { points: Vec<Vec2> },
    /// SVG path data; only the first subpath is used, arcs are not supported.
    Path { d: String },
}

fn default_inner() -> f64 {
    0.45
}

const CURVE_STEPS: usize = 16;

impl ShapeSpec {
    /// The shape as a ring in the unit box.
    pub fn ring(&self) -> Result<Ring, String> {
        use std::f64::consts::{PI, TAU};
        let around = |n: usize, r: &dyn Fn(usize) -> f64, phase: f64| -> Ring {
            (0..n).map(|k| {
                let a = phase + TAU * k as f64 / n as f64;
                Vec2::new(a.cos() * r(k), a.sin() * r(k))
            }).collect()
        };
        let ring = match self {
            ShapeSpec::Circle => around(128, &|_| 1.0, -PI / 2.0),
            ShapeSpec::Square => vec![Vec2::new(-0.85, -0.85), Vec2::new(0.85, -0.85), Vec2::new(0.85, 0.85), Vec2::new(-0.85, 0.85)],
            ShapeSpec::Triangle => around(3, &|_| 1.0, -PI / 2.0),
            ShapeSpec::Polygon { sides } => {
                if *sides < 3 {
                    return Err(format!("a polygon needs at least 3 sides, got {sides}"));
                }
                around(*sides, &|_| 1.0, -PI / 2.0)
            }
            ShapeSpec::Star { points, inner } => {
                if *points < 2 || !(0.05..1.0).contains(inner) {
                    return Err(format!("star needs points >= 2 and 0.05 <= inner < 1, got {points}, {inner}"));
                }
                let inner = *inner;
                around(points * 2, &move |k| if k % 2 == 0 { 1.0 } else { inner }, -PI / 2.0)
            }
            ShapeSpec::Heart => {
                // The classic parametric heart, scaled into the unit box.
                let pts: Ring = (0..160).map(|k| {
                    let t = TAU * k as f64 / 160.0;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    Vec2::new(x, -y)
                }).collect();
                fit_unit(&pts)
            }
            ShapeSpec::Points { points } => points.clone(),
            ShapeSpec::Path { d } => fit_unit(&parse_path(d)?),
        };
        if ring.len() < 3 {
            return Err("a shape needs at least 3 points".into());
        }
        Ok(ring)
    }
}

/// Signed area (positive for clockwise on screen, where y points down).
pub fn signed_area(ring: &[Vec2]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i].cross(ring[(i + 1) % n])).sum::<f64>() / 2.0
}

/// Scale and centre a ring so its bounding box fits -1..1, keeping its aspect.
pub fn fit_unit(ring: &[Vec2]) -> Ring {
    let (mut lo, mut hi) = (Vec2::new(f64::MAX, f64::MAX), Vec2::new(f64::MIN, f64::MIN));
    for p in ring {
        lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let centre = lo.lerp(hi, 0.5);
    let half = ((hi.x - lo.x).max(hi.y - lo.y) / 2.0).max(1e-9);
    ring.iter().map(|&p| (p - centre) * (1.0 / half)).collect()
}

/// `n` points spaced evenly along the ring's perimeter, starting at its first point.
pub fn resample(ring: &[Vec2], n: usize) -> Ring {
    let m = ring.len();
    let edges: Vec<f64> = (0..m).map(|i| (ring[(i + 1) % m] - ring[i]).len()).collect();
    let total: f64 = edges.iter().sum();
    if total <= 0.0 {
        return vec![ring[0]; n];
    }
    let mut out = Vec::with_capacity(n);
    let (mut edge, mut walked) = (0usize, 0.0);
    for k in 0..n {
        let target = total * k as f64 / n as f64;
        while edge < m - 1 && walked + edges[edge] < target {
            walked += edges[edge];
            edge += 1;
        }
        let t = if edges[edge] > 0.0 { ((target - walked) / edges[edge]).clamp(0.0, 1.0) } else { 0.0 };
        out.push(ring[edge].lerp(ring[(edge + 1) % m], t));
    }
    out
}

/// Two rings of `n` points with the same winding, `b` rotated to line up with `a`.
pub fn align(a: &[Vec2], b: &[Vec2], n: usize) -> (Ring, Ring) {
    let ra = resample(a, n);
    let mut rb = resample(b, n);
    if signed_area(&ra).signum() != signed_area(&rb).signum() {
        rb.reverse();
    }
    let cost = |offset: usize| -> f64 { (0..n).map(|i| (ra[i] - rb[(i + offset) % n]).len_sq()).sum() };
    let best = (0..n).min_by(|&x, &y| cost(x).total_cmp(&cost(y))).unwrap_or(0);
    rb.rotate_left(best);
    (ra, rb)
}

/// The in-between ring at `t` in 0..1.
pub fn interpolate(a: &[Vec2], b: &[Vec2], t: f64) -> Ring {
    a.iter().zip(b).map(|(&p, &q)| p.lerp(q, t)).collect()
}

/// Cubic ease in and out.
pub fn ease_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// Flatten the first subpath of SVG path data into a ring.
pub fn parse_path(d: &str) -> Result<Ring, String> {
    let tokens = tokenize(d)?;
    let mut ring: Ring = Vec::new();
    let mut cur = Vec2::ZERO;
    let mut i = 0;
    let mut cmd = ' ';
    let num = |i: &mut usize| -> Result<f64, String> {
        match tokens.get(*i) {
            Some(Token::Num(v)) => {
                *i += 1;
                Ok(*v)
            }
            _ => Err(format!("path {d:?}: expected a number")),
        }
    };
    while i < tokens.len() {
        if let Token::Cmd(c) = tokens[i] {
            cmd = c;
            i += 1;
            if c == 'Z' || c == 'z' {
                // First subpath only.
                break;
            }
        } else if cmd == ' ' {
            return Err(format!("path {d:?} must start with M"));
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { cur } else { Vec2::ZERO };
        match cmd.to_ascii_uppercase() {
            'M' => {
                if !ring.is_empty() {
                    break;
                }
                cur = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                ring.push(cur);
                // Further coordinate pairs after M are implicit L.
                cmd = if rel { 'l' } else { 'L' };
            }
            'L' => {
                cur = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                ring.push(cur);
            }
            'H' => {
                let x = num(&mut i)?;
                cur = Vec2::new(if rel { cur.x + x } else { x }, cur.y);
                ring.push(cur);
            }
            'V' => {
                let y = num(&mut i)?;
                cur = Vec2::new(cur.x, if rel { cur.y + y } else { y });
                ring.push(cur);
            }
            'C' => {
                let c1 = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                let c2 = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                let end = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                for k in 1..=CURVE_STEPS {
                    let t = k as f64 / CURVE_STEPS as f64;
                    let u = 1.0 - t;
                    ring.push(cur * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + end * (t * t * t));
                }
                cur = end;
            }
            'Q' => {
                let c1 = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                let end = base + Vec2::new(num(&mut i)?, num(&mut i)?);
                for k in 1..=CURVE_STEPS {
                    let t = k as f64 / CURVE_STEPS as f64;
                    let u = 1.0 - t;
                    ring.push(cur * (u * u) + c1 * (2.0 * u * t) + end * (t * t));
                }
                cur = end;
            }
            other => return Err(format!("path command '{other}' is not supported (use M, L, H, V, C, Q, Z)")),
        }
    }
    // Drop a closing point that repeats the start.
    if ring.len() > 1 && (ring[ring.len() - 1] - ring[0]).len() < 1e-9 {
        ring.pop();
    }
    if ring.len() < 3 {
        return Err(format!("path {d:?} has fewer than 3 points"));
    }
    Ok(ring)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Cmd(char),
    Num(f64),
}

fn tokenize(d: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = d.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            out.push(Token::Cmd(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            i += 1;
            let mut seen_dot = c == '.';
            while i < chars.len() {
                let ch = chars[i];
                let exp_sign = (ch == '-' || ch == '+') && matches!(chars[i - 1], 'e' | 'E');
                if ch.is_ascii_digit() || ch == 'e' || ch == 'E' || exp_sign || (ch == '.' && !seen_dot) {
                    seen_dot |= ch == '.';
                    i += 1;
                } else {
                    break;
                }
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Token::Num(s.parse().map_err(|_| format!("bad number {s:?} in path"))?));
        } else if c.is_whitespace() || c == ',' {
            i += 1;
        } else {
            return Err(format!("unexpected {c:?} in path data"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec2, b: Vec2) -> bool {
        (a - b).len() < 1e-9
    }

    #[test]
    fn resampling_spaces_points_evenly_along_the_perimeter() {
        let sq = vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)];
        let r = resample(&sq, 8);
        assert_eq!(r.len(), 8);
        assert!(approx(r[0], Vec2::new(0.0, 0.0)));
        assert!(approx(r[1], Vec2::new(0.5, 0.0)));
        assert!(approx(r[2], Vec2::new(1.0, 0.0)));
        assert!(approx(r[5], Vec2::new(0.5, 1.0)));
        assert!(approx(r[7], Vec2::new(0.0, 0.5)));
    }

    #[test]
    fn alignment_undoes_a_rotation_and_a_reversed_winding() {
        let a = ShapeSpec::Star { points: 5, inner: 0.4 }.ring().unwrap();
        let mut b = a.clone();
        b.rotate_left(3);
        b.reverse();
        let (ra, rb) = align(&a, &b, 10);
        // Same shape: once aligned, every point should match.
        for (p, q) in ra.iter().zip(&rb) {
            assert!((*p - *q).len() < 1e-9, "{p:?} vs {q:?}");
        }
    }

    #[test]
    fn interpolation_hits_both_ends_and_passes_through_the_middle() {
        let a = ShapeSpec::Circle.ring().unwrap();
        let b = ShapeSpec::Square.ring().unwrap();
        let (ra, rb) = align(&a, &b, 64);
        assert_eq!(interpolate(&ra, &rb, 0.0), ra);
        assert_eq!(interpolate(&ra, &rb, 1.0), rb);
        let mid = interpolate(&ra, &rb, 0.5);
        for i in 0..64 {
            assert!(approx(mid[i], ra[i].lerp(rb[i], 0.5)));
        }
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn svg_paths_parse_relative_commands_and_curves() {
        // A 10x10 square drawn with relative commands.
        let r = parse_path("m 0 0 h 10 v 10 h -10 z").unwrap();
        assert_eq!(r, vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0), Vec2::new(0.0, 10.0)]);
        // Implicit lineto after M, scientific notation, packed minus signs.
        let r = parse_path("M0,0 10,0 10-10z").unwrap();
        assert_eq!(r[2], Vec2::new(10.0, -10.0));
        let r = parse_path("M1e1 0L0 0 5 5Z").unwrap();
        assert_eq!(r[0], Vec2::new(10.0, 0.0));
        // A cubic is flattened into CURVE_STEPS points ending at its end point.
        let r = parse_path("M0 0 C 0 10 10 10 10 0 Z").unwrap();
        assert_eq!(r.len(), 1 + CURVE_STEPS);
        assert!(approx(*r.last().unwrap(), Vec2::new(10.0, 0.0)));
        assert!(parse_path("M0 0 A 5 5 0 0 1 10 0 Z").unwrap_err().contains("not supported"));
        assert!(parse_path("0 0 L 1 1").is_err());
        // Only the first subpath is kept.
        assert_eq!(parse_path("M0 0 H4 V4 Z M10 10 H20 V20 Z").unwrap().len(), 3);
    }

    #[test]
    fn shapes_from_specs_fit_the_unit_box() {
        let heart = ShapeSpec::Heart.ring().unwrap();
        let maxc = heart.iter().map(|p| p.x.abs().max(p.y.abs())).fold(0.0, f64::max);
        assert!((maxc - 1.0).abs() < 0.02, "{maxc}");
        assert!(ShapeSpec::Polygon { sides: 2 }.ring().is_err());
        let spec: ShapeSpec = toml::from_str("shape = \"star\"\npoints = 6").unwrap();
        assert_eq!(spec, ShapeSpec::Star { points: 6, inner: 0.45 });
        let path = ShapeSpec::Path { d: "M 100 100 H 300 V 200 H 100 Z".into() }.ring().unwrap();
        assert!(approx(path[0], Vec2::new(-1.0, -0.5)));
    }
}
