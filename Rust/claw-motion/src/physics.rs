//! Deterministic 2D rigid-body physics for circles against static capsules.
//!
//! The idea is Matter.js's (liabru/matter-js, MIT): a fixed timestep, an
//! iterative impulse solver for velocities, then a positional correction pass so
//! stacked bodies do not sink. This is a fresh, much smaller implementation for
//! one job: render "things fall and collide" footage frame by frame, and report
//! every collision with its time and speed so a sound can land on the exact
//! frame of the hit. No rotation (bodies are circles), no sleeping, no joints.
//!
//! Determinism: plain `f64` arithmetic in a fixed order (bodies by index, pairs
//! by index), a seeded RNG for spawns and no hash-map iteration, so the same
//! scene gives the same frames and the same impacts on every run.

use std::collections::{BTreeSet, HashMap};

use serde::Serialize;

use crate::geom::{Rgb, Vec2};

/// A dynamic circle.
#[derive(Debug, Clone)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f64,
    pub inv_mass: f64,
    pub color: Rgb,
    /// False once the body has fallen out of the world; it is no longer simulated.
    pub alive: bool,
}

impl Body {
    /// Mass proportional to area, so big balls push small ones around.
    pub fn new(pos: Vec2, vel: Vec2, radius: f64, color: Rgb) -> Body {
        let area = std::f64::consts::PI * radius * radius;
        Body { pos, vel, radius, inv_mass: 1.0 / area.max(1e-9), color, alive: true }
    }
}

/// A static segment with rounded ends (radius 0 at both ends of a point is a peg).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capsule {
    pub a: Vec2,
    pub b: Vec2,
    pub radius: f64,
}

impl Capsule {
    /// Closest point on the core segment to `p`.
    pub fn closest(&self, p: Vec2) -> Vec2 {
        let ab = self.b - self.a;
        let denom = ab.len_sq();
        if denom < 1e-12 {
            return self.a;
        }
        let t = ((p - self.a).dot(ab) / denom).clamp(0.0, 1.0);
        self.a + ab * t
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub gravity: Vec2,
    pub restitution: f64,
    pub friction: f64,
    /// Fraction of velocity lost per second to air drag.
    pub air: f64,
    pub velocity_iterations: usize,
    pub position_iterations: usize,
    /// Speed cap in px/s, so a body cannot tunnel through a thin wall in one step.
    pub max_speed: f64,
    /// Approach speed (px/s) a new contact needs to count as an impact.
    pub min_impact_speed: f64,
    /// Below this approach speed contacts do not bounce (stops jitter at rest).
    pub bounce_threshold: f64,
    /// Bodies below this y (px) are retired.
    pub kill_y: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            gravity: Vec2::new(0.0, 1960.0),
            restitution: 0.4,
            friction: 0.08,
            air: 0.02,
            velocity_iterations: 8,
            position_iterations: 3,
            max_speed: 3200.0,
            min_impact_speed: 140.0,
            bounce_threshold: 60.0,
            kill_y: 4000.0,
        }
    }
}

/// One new contact that hit hard enough to be heard.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Impact {
    /// Seconds from the start of the simulation.
    pub time: f64,
    /// Approach speed along the contact normal, px/s.
    pub speed: f64,
    pub x: f64,
    pub y: f64,
    /// Radius of the dynamic body that was hit (the smaller one for pairs); sets pitch.
    pub radius: f64,
    /// True when the other side was a wall or peg.
    pub on_static: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Other {
    Body(usize),
    Static(usize),
}

#[derive(Debug, Clone, Copy)]
struct Contact {
    a: usize,
    other: Other,
    /// Unit normal pointing from the other side toward `a`.
    normal: Vec2,
    /// Target normal speed after the solve (restitution bounce).
    bounce: f64,
    jn: f64,
    jt: f64,
}

/// Static geometry bucketed into a fixed grid, so each body only tests nearby walls and pegs.
#[derive(Debug, Clone, Default)]
struct StaticGrid {
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl StaticGrid {
    fn build(statics: &[Capsule], cell: f64) -> StaticGrid {
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, s) in statics.iter().enumerate() {
            let (lo, hi) = aabb_cells(s.a.x.min(s.b.x) - s.radius, s.a.y.min(s.b.y) - s.radius, s.a.x.max(s.b.x) + s.radius, s.a.y.max(s.b.y) + s.radius, cell);
            for cx in lo.0..=hi.0 {
                for cy in lo.1..=hi.1 {
                    cells.entry((cx, cy)).or_default().push(i);
                }
            }
        }
        StaticGrid { cell, cells }
    }

    /// Static indices near a circle, sorted and unique (a fixed order keeps the solve deterministic).
    fn near(&self, p: Vec2, r: f64, out: &mut Vec<usize>) {
        out.clear();
        let (lo, hi) = aabb_cells(p.x - r, p.y - r, p.x + r, p.y + r, self.cell);
        for cx in lo.0..=hi.0 {
            for cy in lo.1..=hi.1 {
                if let Some(list) = self.cells.get(&(cx, cy)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

fn aabb_cells(x0: f64, y0: f64, x1: f64, y1: f64, cell: f64) -> ((i64, i64), (i64, i64)) {
    let c = |v: f64| (v / cell).floor() as i64;
    ((c(x0), c(y0)), (c(x1), c(y1)))
}

pub struct World {
    pub settings: Settings,
    pub bodies: Vec<Body>,
    pub statics: Vec<Capsule>,
    pub time: f64,
    pub impacts: Vec<Impact>,
    static_grid: StaticGrid,
    /// Pairs touching after the previous step; a contact is only an impact the step it begins.
    touching: BTreeSet<(usize, Other)>,
}

impl World {
    pub fn new(settings: Settings, statics: Vec<Capsule>) -> World {
        let grid = StaticGrid::build(&statics, 64.0);
        World { settings, bodies: Vec::new(), statics, time: 0.0, impacts: Vec::new(), static_grid: grid, touching: BTreeSet::new() }
    }

    pub fn add(&mut self, body: Body) {
        self.bodies.push(body);
    }

    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter().filter(|b| b.alive)
    }

    fn detect(&self) -> Vec<(usize, Other, Vec2, f64)> {
        let mut found = Vec::new();
        let max_r = self.alive().map(|b| b.radius).fold(1.0, f64::max);
        let cell = 2.0 * max_r;
        let key = |p: Vec2| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
        let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, b) in self.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            grid.entry(key(b.pos)).or_default().push(i);
        }
        let mut near = Vec::new();
        for (i, a) in self.bodies.iter().enumerate().filter(|(_, b)| b.alive) {
            let (kx, ky) = key(a.pos);
            let mut pairs: Vec<usize> = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(list) = grid.get(&(kx + dx, ky + dy)) {
                        pairs.extend(list.iter().copied().filter(|&j| j > i));
                    }
                }
            }
            pairs.sort_unstable();
            for j in pairs {
                let b = &self.bodies[j];
                let d = a.pos - b.pos;
                let reach = a.radius + b.radius;
                if d.len_sq() < reach * reach {
                    let dist = d.len();
                    found.push((i, Other::Body(j), d.normalize_or(Vec2::new(0.0, -1.0)), reach - dist));
                }
            }
            self.static_grid.near(a.pos, a.radius, &mut near);
            for &s in &near {
                let cap = &self.statics[s];
                let d = a.pos - cap.closest(a.pos);
                let reach = a.radius + cap.radius;
                if d.len_sq() < reach * reach {
                    found.push((i, Other::Static(s), d.normalize_or(Vec2::new(0.0, -1.0)), reach - d.len()));
                }
            }
        }
        found
    }

    fn other_vel(&self, o: Other) -> Vec2 {
        match o {
            Other::Body(j) => self.bodies[j].vel,
            Other::Static(_) => Vec2::ZERO,
        }
    }

    fn other_inv_mass(&self, o: Other) -> f64 {
        match o {
            Other::Body(j) => self.bodies[j].inv_mass,
            Other::Static(_) => 0.0,
        }
    }

    /// Advance by `dt` seconds.
    pub fn step(&mut self, dt: f64) {
        let s = self.settings;
        let drag = (1.0 - s.air * dt).max(0.0);
        for b in self.bodies.iter_mut().filter(|b| b.alive) {
            b.vel = (b.vel + s.gravity * dt) * drag;
        }

        let raw = self.detect();
        let mut contacts = Vec::with_capacity(raw.len());
        let mut now_touching = BTreeSet::new();
        for (a, other, normal, _depth) in raw {
            let vn = (self.bodies[a].vel - self.other_vel(other)).dot(normal);
            let bounce = if vn < -s.bounce_threshold { -s.restitution * vn } else { 0.0 };
            if vn < -s.min_impact_speed && !self.touching.contains(&(a, other)) {
                let (radius, on_static) = match other {
                    Other::Body(j) => (self.bodies[a].radius.min(self.bodies[j].radius), false),
                    Other::Static(_) => (self.bodies[a].radius, true),
                };
                let p = self.bodies[a].pos - normal * self.bodies[a].radius;
                self.impacts.push(Impact { time: self.time, speed: -vn, x: p.x, y: p.y, radius, on_static });
            }
            now_touching.insert((a, other));
            contacts.push(Contact { a, other, normal, bounce, jn: 0.0, jt: 0.0 });
        }
        self.touching = now_touching;

        for _ in 0..s.velocity_iterations {
            for c in contacts.iter_mut() {
                let inv = self.bodies[c.a].inv_mass + match c.other {
                    Other::Body(j) => self.bodies[j].inv_mass,
                    Other::Static(_) => 0.0,
                };
                if inv <= 0.0 {
                    continue;
                }
                let other_vel = match c.other {
                    Other::Body(j) => self.bodies[j].vel,
                    Other::Static(_) => Vec2::ZERO,
                };
                let rel = self.bodies[c.a].vel - other_vel;
                let vn = rel.dot(c.normal);
                let jn = (c.jn + (c.bounce - vn) / inv).max(0.0);
                let dn = jn - c.jn;
                c.jn = jn;
                let (a, o, n) = (c.a, c.other, c.normal);
                apply_impulse(&mut self.bodies, a, o, n * dn);

                let other_vel = match o {
                    Other::Body(j) => self.bodies[j].vel,
                    Other::Static(_) => Vec2::ZERO,
                };
                let tangent = n.perp();
                let vt = (self.bodies[a].vel - other_vel).dot(tangent);
                let limit = s.friction * c.jn;
                let jt = (c.jt - vt / inv).clamp(-limit, limit);
                let dt_imp = jt - c.jt;
                c.jt = jt;
                apply_impulse(&mut self.bodies, a, o, tangent * dt_imp);
            }
        }

        for b in self.bodies.iter_mut().filter(|b| b.alive) {
            let speed = b.vel.len();
            if speed > s.max_speed {
                b.vel = b.vel * (s.max_speed / speed);
            }
            b.pos += b.vel * dt;
            if b.pos.y > s.kill_y {
                b.alive = false;
            }
        }

        // Positional correction: push overlapping pairs apart (Matter.js's
        // position solver idea), re-measuring depth each pass.
        const SLOP: f64 = 0.3;
        const SHARE: f64 = 0.8;
        for _ in 0..s.position_iterations {
            for c in &contacts {
                let a = c.a;
                if !self.bodies[a].alive {
                    continue;
                }
                let (normal, depth) = match c.other {
                    Other::Body(j) => {
                        if !self.bodies[j].alive {
                            continue;
                        }
                        let d = self.bodies[a].pos - self.bodies[j].pos;
                        (d.normalize_or(c.normal), self.bodies[a].radius + self.bodies[j].radius - d.len())
                    }
                    Other::Static(k) => {
                        let cap = self.statics[k];
                        let d = self.bodies[a].pos - cap.closest(self.bodies[a].pos);
                        (d.normalize_or(c.normal), self.bodies[a].radius + cap.radius - d.len())
                    }
                };
                if depth <= SLOP {
                    continue;
                }
                let inv = self.bodies[a].inv_mass + self.other_inv_mass(c.other);
                let push = (depth - SLOP) * SHARE / inv;
                let ia = self.bodies[a].inv_mass;
                self.bodies[a].pos += normal * (push * ia);
                if let Other::Body(j) = c.other {
                    let ib = self.bodies[j].inv_mass;
                    self.bodies[j].pos -= normal * (push * ib);
                }
            }
        }
        self.time += dt;
    }
}

fn apply_impulse(bodies: &mut [Body], a: usize, o: Other, impulse: Vec2) {
    let ia = bodies[a].inv_mass;
    bodies[a].vel += impulse * ia;
    if let Other::Body(j) = o {
        let ib = bodies[j].inv_mass;
        bodies[j].vel -= impulse * ib;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 240.0;
    const RED: Rgb = Rgb(255, 0, 0);

    fn floor() -> Capsule {
        Capsule { a: Vec2::new(-1000.0, 1000.0), b: Vec2::new(2000.0, 1000.0), radius: 5.0 }
    }

    #[test]
    fn a_dropped_ball_lands_once_and_comes_to_rest_on_the_floor() {
        let mut w = World::new(Settings::default(), vec![floor()]);
        w.add(Body::new(Vec2::new(500.0, 200.0), Vec2::ZERO, 20.0, RED));
        for _ in 0..(240 * 4) {
            w.step(DT);
        }
        let b = &w.bodies[0];
        // Resting on top of the floor: centre at 1000 - 5 - 20, within the slop.
        assert!((b.pos.y - 975.0).abs() < 1.5, "y = {}", b.pos.y);
        assert!(b.vel.len() < 5.0, "still moving at {}", b.vel.len());
        assert!(!w.impacts.is_empty());
        // Free fall of 775 px under 1960 px/s^2 takes sqrt(2*775/1960) = 0.889 s.
        let first = w.impacts[0];
        assert!((first.time - 0.889).abs() < 0.02, "first impact at {}", first.time);
        assert!((first.speed - (2.0f64 * 1960.0 * 775.0).sqrt()).abs() < 40.0);
        assert!(first.on_static);
        // Bounces decay; the impacts stop once it settles.
        assert!(w.impacts.iter().all(|i| i.time < 2.5), "{:?}", w.impacts.last());
        assert!(w.impacts.windows(2).all(|p| p[1].speed < p[0].speed));
    }

    #[test]
    fn restitution_sets_the_bounce_height() {
        let bounce_peak = |e: f64| {
            let s = Settings { restitution: e, air: 0.0, ..Settings::default() };
            let mut w = World::new(s, vec![floor()]);
            w.add(Body::new(Vec2::new(500.0, 575.0), Vec2::ZERO, 20.0, RED));
            let mut landed = false;
            let mut peak = f64::MAX;
            for _ in 0..(240 * 2) {
                w.step(DT);
                if !w.impacts.is_empty() {
                    landed = true;
                }
                if landed && w.bodies[0].vel.y > 0.0 && peak < f64::MAX {
                    break;
                }
                if landed {
                    peak = peak.min(w.bodies[0].pos.y);
                }
            }
            975.0 - peak
        };
        // Drop height 400 px; a bounce reaches about e^2 of it.
        let h = bounce_peak(0.5);
        assert!((h - 100.0).abs() < 20.0, "bounce height {h}");
        assert!(bounce_peak(0.0) < 3.0);
    }

    #[test]
    fn stacked_balls_do_not_sink_into_each_other() {
        let mut w = World::new(Settings::default(), vec![floor()]);
        for k in 0..5 {
            w.add(Body::new(Vec2::new(500.0, 900.0 - 45.0 * k as f64), Vec2::ZERO, 20.0, RED));
        }
        for _ in 0..(240 * 5) {
            w.step(DT);
        }
        // A perfect column is unstable; whatever pile results, nothing overlaps much.
        for i in 0..5 {
            for j in (i + 1)..5 {
                let d = (w.bodies[i].pos - w.bodies[j].pos).len();
                assert!(d > 40.0 - 2.0, "bodies {i},{j} overlap: {d}");
            }
            assert!(w.bodies[i].pos.y <= 975.0 + 1.5);
        }
    }

    #[test]
    fn equal_balls_swap_velocities_in_a_head_on_elastic_hit() {
        let s = Settings { gravity: Vec2::ZERO, restitution: 1.0, air: 0.0, friction: 0.0, ..Settings::default() };
        let mut w = World::new(s, vec![]);
        w.add(Body::new(Vec2::new(0.0, 0.0), Vec2::new(500.0, 0.0), 10.0, RED));
        w.add(Body::new(Vec2::new(100.0, 0.0), Vec2::ZERO, 10.0, RED));
        for _ in 0..120 {
            w.step(DT);
        }
        assert!(w.bodies[0].vel.x.abs() < 5.0, "{:?}", w.bodies[0].vel);
        assert!((w.bodies[1].vel.x - 500.0).abs() < 5.0, "{:?}", w.bodies[1].vel);
        assert_eq!(w.impacts.len(), 1);
        assert!(!w.impacts[0].on_static);
    }

    #[test]
    fn a_ball_rolls_off_a_slope_and_falls_out_of_the_world() {
        let ramp = Capsule { a: Vec2::new(0.0, 0.0), b: Vec2::new(400.0, 200.0), radius: 4.0 };
        let s = Settings { kill_y: 1500.0, ..Settings::default() };
        let mut w = World::new(s, vec![ramp]);
        w.add(Body::new(Vec2::new(50.0, -40.0), Vec2::ZERO, 12.0, RED));
        for _ in 0..(240 * 3) {
            w.step(DT);
        }
        assert!(!w.bodies[0].alive);
        assert_eq!(w.alive().count(), 0);
    }

    #[test]
    fn identical_worlds_give_identical_results() {
        let run = || {
            let mut w = World::new(Settings::default(), vec![floor()]);
            for k in 0..40 {
                w.add(Body::new(Vec2::new(400.0 + (k % 7) as f64 * 13.0, 100.0 - k as f64 * 30.0), Vec2::ZERO, 8.0 + (k % 3) as f64 * 4.0, RED));
            }
            for _ in 0..(240 * 3) {
                w.step(DT);
            }
            (w.bodies.iter().map(|b| (b.pos.x.to_bits(), b.pos.y.to_bits())).collect::<Vec<_>>(), w.impacts.len())
        };
        assert_eq!(run(), run());
    }
}
