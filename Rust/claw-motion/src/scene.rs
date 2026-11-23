//! Scene files (TOML) and the frame-by-frame drivers that turn them into pictures.

use std::path::Path;

use serde::Deserialize;

use crate::geom::{Rgb, Rng, Vec2};
use crate::morph::{self, Ring, ShapeSpec};
use crate::physics::{Body, Capsule, Settings, World};
use crate::raster::Frame;
use crate::sfx::SfxSettings;

/// Longest scene accepted, seconds; Shorts are under three minutes.
pub const MAX_DURATION: f64 = 180.0;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Video {
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_fps")]
    pub fps: u32,
    pub duration: f64,
    #[serde(default = "default_background")]
    pub background: Rgb,
}

fn default_width() -> u32 {
    1080
}
fn default_height() -> u32 {
    1920
}
fn default_fps() -> u32 {
    30
}
fn default_background() -> Rgb {
    Rgb(0x11, 0x13, 0x18)
}

impl Video {
    pub fn validate(&self) -> Result<(), String> {
        if self.width % 2 != 0 || self.height % 2 != 0 || self.width < 16 || self.height < 16 {
            return Err(format!("video size {}x{} must be even and at least 16 px (H.264 4:2:0)", self.width, self.height));
        }
        if !(1..=120).contains(&self.fps) {
            return Err(format!("fps {} must be 1-120", self.fps));
        }
        if !(self.duration > 0.0 && self.duration <= MAX_DURATION) {
            return Err(format!("duration {} must be in (0, {MAX_DURATION}] seconds", self.duration));
        }
        Ok(())
    }

    pub fn frames(&self) -> u64 {
        (self.duration * f64::from(self.fps)).round() as u64
    }
}

// ------------------------------------------------------------------ physics

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsScene {
    pub video: Video,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub physics: PhysicsSettings,
    #[serde(default)]
    pub sfx: SfxSpec,
    #[serde(default, rename = "wall")]
    pub walls: Vec<WallSpec>,
    #[serde(default, rename = "pegs")]
    pub pegs: Vec<PegGrid>,
    #[serde(default, rename = "emitter")]
    pub emitters: Vec<Emitter>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PhysicsSettings {
    pub gravity: Vec2,
    pub restitution: f64,
    pub friction: f64,
    pub air: f64,
    /// Simulation steps per video frame.
    pub substeps: u32,
    pub min_impact_speed: f64,
    pub max_speed: f64,
    /// Static geometry color.
    pub color: Rgb,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        let s = Settings::default();
        PhysicsSettings {
            gravity: s.gravity,
            restitution: s.restitution,
            friction: s.friction,
            air: s.air,
            substeps: 8,
            min_impact_speed: s.min_impact_speed,
            max_speed: s.max_speed,
            color: Rgb(0xe8, 0xe6, 0xe3),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SfxSpec {
    pub enabled: bool,
    pub base_hz: f64,
    pub full_speed: f64,
    pub per_window: usize,
}

impl Default for SfxSpec {
    fn default() -> Self {
        let s = SfxSettings::default();
        SfxSpec { enabled: true, base_hz: s.base_hz, full_speed: s.full_speed, per_window: s.per_window }
    }
}

impl SfxSpec {
    pub fn settings(&self) -> SfxSettings {
        SfxSettings { base_hz: self.base_hz, full_speed: self.full_speed, per_window: self.per_window.max(1), ..SfxSettings::default() }
    }
}

/// A static wall: a polyline whose segments become capsules.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallSpec {
    pub points: Vec<Vec2>,
    #[serde(default = "default_wall_radius")]
    pub radius: f64,
    #[serde(default)]
    pub closed: bool,
}

fn default_wall_radius() -> f64 {
    6.0
}

/// A grid of round pegs (a Plinko board when `stagger` is on).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PegGrid {
    pub origin: Vec2,
    pub rows: u32,
    pub cols: u32,
    pub spacing: Vec2,
    #[serde(default = "default_peg_radius")]
    pub radius: f64,
    #[serde(default = "default_true")]
    pub stagger: bool,
}

fn default_peg_radius() -> f64 {
    7.0
}
fn default_true() -> bool {
    true
}

/// Drops `count` balls from `at`, one every `every` seconds starting at `start`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emitter {
    pub at: Vec2,
    pub count: u32,
    #[serde(default = "default_every")]
    pub every: f64,
    #[serde(default)]
    pub start: f64,
    /// Horizontal spread of spawn positions, px either side of `at`.
    #[serde(default)]
    pub jitter: f64,
    #[serde(default)]
    pub velocity: Vec2,
    /// Radius range [min, max] in px.
    pub radius: [f64; 2],
    pub colors: Vec<Rgb>,
}

fn default_every() -> f64 {
    0.1
}

impl PhysicsScene {
    pub fn load(path: &Path) -> Result<PhysicsScene, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        PhysicsScene::parse(&text)
    }

    pub fn parse(text: &str) -> Result<PhysicsScene, String> {
        let scene: PhysicsScene = toml::from_str(text).map_err(|e| format!("scene: {e}"))?;
        scene.validate()?;
        Ok(scene)
    }

    fn validate(&self) -> Result<(), String> {
        self.video.validate()?;
        if self.physics.substeps == 0 || self.physics.substeps > 64 {
            return Err("physics.substeps must be 1-64".into());
        }
        for (i, e) in self.emitters.iter().enumerate() {
            if e.colors.is_empty() {
                return Err(format!("emitter {i}: colors is empty"));
            }
            if !(e.radius[0] > 0.0 && e.radius[0] <= e.radius[1]) {
                return Err(format!("emitter {i}: radius must be [min, max] with 0 < min <= max"));
            }
            if e.every <= 0.0 {
                return Err(format!("emitter {i}: every must be > 0"));
            }
        }
        for (i, w) in self.walls.iter().enumerate() {
            if w.points.len() < 2 {
                return Err(format!("wall {i}: needs at least 2 points"));
            }
        }
        if self.emitters.iter().map(|e| u64::from(e.count)).sum::<u64>() > 5000 {
            return Err("more than 5000 bodies; the solver is meant for hundreds".into());
        }
        Ok(())
    }

    pub fn statics(&self) -> Vec<Capsule> {
        let mut out = Vec::new();
        for w in &self.walls {
            let n = w.points.len();
            let segs = if w.closed { n } else { n - 1 };
            for i in 0..segs {
                out.push(Capsule { a: w.points[i], b: w.points[(i + 1) % n], radius: w.radius });
            }
        }
        for g in &self.pegs {
            for r in 0..g.rows {
                let shift = if g.stagger && r % 2 == 1 { g.spacing.x / 2.0 } else { 0.0 };
                let cols = if g.stagger && r % 2 == 1 { g.cols.saturating_sub(1) } else { g.cols };
                for c in 0..cols {
                    let p = g.origin + Vec2::new(shift + g.spacing.x * f64::from(c), g.spacing.y * f64::from(r));
                    out.push(Capsule { a: p, b: p, radius: g.radius });
                }
            }
        }
        out
    }

    pub fn settings(&self) -> Settings {
        let p = &self.physics;
        Settings {
            gravity: p.gravity,
            restitution: p.restitution,
            friction: p.friction,
            air: p.air,
            max_speed: p.max_speed,
            min_impact_speed: p.min_impact_speed,
            kill_y: f64::from(self.video.height) + 400.0,
            ..Settings::default()
        }
    }
}

/// One planned spawn, in time order.
#[derive(Debug, Clone)]
struct Spawn {
    time: f64,
    body: Body,
}

/// Steps a physics scene frame by frame and draws it.
pub struct PhysicsRun {
    pub scene: PhysicsScene,
    pub world: World,
    spawns: Vec<Spawn>,
    next_spawn: usize,
    pub spawned: usize,
    frame_index: u64,
}

impl PhysicsRun {
    pub fn new(scene: PhysicsScene) -> PhysicsRun {
        let mut rng = Rng::new(scene.seed);
        let mut spawns = Vec::new();
        for e in &scene.emitters {
            for k in 0..e.count {
                // Always draw the same number of values per spawn, so changing
                // one emitter's jitter does not reshuffle every later body.
                let x = e.jitter * (2.0 * rng.unit() - 1.0);
                let r = e.radius[0] + (e.radius[1] - e.radius[0]) * rng.unit();
                let color = e.colors[(rng.next_u64() % e.colors.len() as u64) as usize];
                spawns.push(Spawn {
                    time: e.start + e.every * f64::from(k),
                    body: Body::new(e.at + Vec2::new(x, 0.0), e.velocity, r, color),
                });
            }
        }
        // Stable sort keeps emitter order for equal times.
        spawns.sort_by(|a, b| a.time.total_cmp(&b.time));
        let world = World::new(scene.settings(), scene.statics());
        PhysicsRun { scene, world, spawns, next_spawn: 0, spawned: 0, frame_index: 0 }
    }

    /// Advance the simulation to the time of the next frame.
    pub fn advance(&mut self) {
        let fps = f64::from(self.scene.video.fps);
        let sub = self.scene.physics.substeps;
        let dt = 1.0 / (fps * f64::from(sub));
        if self.frame_index > 0 {
            for _ in 0..sub {
                while self.next_spawn < self.spawns.len() && self.spawns[self.next_spawn].time <= self.world.time + 1e-9 {
                    self.world.add(self.spawns[self.next_spawn].body.clone());
                    self.next_spawn += 1;
                    self.spawned += 1;
                }
                self.world.step(dt);
            }
        }
        self.frame_index += 1;
    }

    pub fn draw(&self, frame: &mut Frame) {
        frame.clear(self.scene.video.background);
        for s in &self.world.statics {
            frame.capsule(s.a, s.b, s.radius, self.scene.physics.color);
        }
        for b in self.world.alive() {
            frame.circle(b.pos, b.radius, b.color, Some((b.color.darken(0.6), 1.5)));
        }
    }
}

// ------------------------------------------------------------------ morph

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphScene {
    pub video: Video,
    pub morph: MorphSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphSpec {
    /// Where the shape's unit box is centred, px.
    pub center: Vec2,
    /// Half-size of the unit box, px.
    pub scale: f64,
    #[serde(default = "default_points")]
    pub points: usize,
    /// Morph from the last shape back to the first at the end.
    #[serde(default)]
    pub r#loop: bool,
    pub shapes: Vec<Keyframe>,
}

fn default_points() -> usize {
    256
}

// No `deny_unknown_fields` here: serde does not support it with `flatten`.
// Typos are still caught, because `ShapeSpec` denies fields it does not know.
#[derive(Debug, Clone, Deserialize)]
pub struct Keyframe {
    #[serde(flatten)]
    pub shape: ShapeSpec,
    pub color: Rgb,
    /// Seconds the shape holds still.
    #[serde(default = "default_hold")]
    pub hold: f64,
    /// Seconds spent morphing into the next shape.
    #[serde(default = "default_morph")]
    pub morph: f64,
}

fn default_hold() -> f64 {
    0.8
}
fn default_morph() -> f64 {
    0.7
}

impl MorphScene {
    pub fn load(path: &Path) -> Result<MorphScene, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        MorphScene::parse(&text)
    }

    pub fn parse(text: &str) -> Result<MorphScene, String> {
        let scene: MorphScene = toml::from_str(text).map_err(|e| format!("scene: {e}"))?;
        scene.video.validate()?;
        if scene.morph.shapes.is_empty() {
            return Err("morph.shapes is empty".into());
        }
        if !(16..=4096).contains(&scene.morph.points) {
            return Err("morph.points must be 16-4096".into());
        }
        Ok(scene)
    }
}

/// Precomputed, aligned rings for every transition of a morph scene.
pub struct MorphRun {
    pub scene: MorphScene,
    rings: Vec<Ring>,
    /// For transition k (shape k -> k+1): both rings aligned to each other.
    pairs: Vec<(Ring, Ring)>,
}

impl MorphRun {
    pub fn new(scene: MorphScene) -> Result<MorphRun, String> {
        let n = scene.morph.points;
        let rings: Vec<Ring> = scene
            .morph
            .shapes
            .iter()
            .enumerate()
            .map(|(i, k)| k.shape.ring().map(|r| morph::resample(&r, n)).map_err(|e| format!("shape {i}: {e}")))
            .collect::<Result<_, _>>()?;
        let count = rings.len();
        let transitions = if scene.morph.r#loop { count } else { count - 1 };
        let pairs = (0..transitions).map(|k| morph::align(&rings[k], &rings[(k + 1) % count], n)).collect();
        Ok(MorphRun { scene, rings, pairs })
    }

    /// The ring (in px) and color at time `t`.
    pub fn at(&self, t: f64) -> (Ring, Rgb) {
        let shapes = &self.scene.morph.shapes;
        let count = shapes.len();
        let mut clock = 0.0;
        let place = |ring: &Ring| -> Ring { ring.iter().map(|&p| self.scene.morph.center + p * self.scene.morph.scale).collect() };
        for k in 0..count {
            let hold_end = clock + shapes[k].hold;
            if t < hold_end || k >= self.pairs.len() {
                return (place(&self.rings[k]), shapes[k].color);
            }
            let morph_end = hold_end + shapes[k].morph;
            if t < morph_end {
                let u = morph::ease_in_out((t - hold_end) / shapes[k].morph.max(1e-9));
                let (a, b) = &self.pairs[k];
                let next = &shapes[(k + 1) % count];
                return (place(&morph::interpolate(a, b, u)), shapes[k].color.lerp(next.color, u));
            }
            clock = morph_end;
        }
        // Past a loop's final morph: back on the first shape.
        (place(&self.rings[0]), shapes[0].color)
    }

    pub fn draw(&self, frame: &mut Frame, t: f64) {
        frame.clear(self.scene.video.background);
        let (ring, color) = self.at(t);
        frame.polygon(&ring, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHYSICS: &str = include_str!("../examples/plinko.toml");
    const MORPH: &str = include_str!("../examples/morph.toml");

    #[test]
    fn the_example_scenes_parse() {
        let p = PhysicsScene::parse(PHYSICS).unwrap();
        assert!(!p.statics().is_empty());
        let m = MorphScene::parse(MORPH).unwrap();
        assert!(m.morph.shapes.len() >= 3);
    }

    #[test]
    fn bad_scenes_are_rejected_with_a_reason() {
        let base = "[video]\nduration = 5\n";
        assert!(PhysicsScene::parse(&format!("{base}[physics]\nsubsteps = 0\n")).unwrap_err().contains("substeps"));
        assert!(PhysicsScene::parse("[video]\nduration = 500\n").unwrap_err().contains("duration"));
        assert!(PhysicsScene::parse("[video]\nduration = 5\nwidth = 1081\n").unwrap_err().contains("even"));
        let e = PhysicsScene::parse(&format!("{base}[[emitter]]\nat = [0, 0]\ncount = 3\nradius = [5, 2]\ncolors = [\"#ffffff\"]\n")).unwrap_err();
        assert!(e.contains("radius"), "{e}");
        assert!(PhysicsScene::parse(&format!("{base}typo = 1\n")).is_err());
        let typo = "[video]\nduration = 2\n[morph]\ncenter = [0, 0]\nscale = 1\n[[morph.shapes]]\nshape = \"star\"\npoint = 5\ncolor = \"#ffffff\"\n";
        assert!(MorphScene::parse(typo).is_err(), "a misspelled shape field must be rejected");
    }

    #[test]
    fn staggered_peg_rows_are_offset_by_half_a_spacing() {
        let s = PhysicsScene::parse("[video]\nduration = 1\n[[pegs]]\norigin = [100, 100]\nrows = 2\ncols = 3\nspacing = [60, 50]\n").unwrap();
        let pegs = s.statics();
        assert_eq!(pegs.len(), 3 + 2);
        assert_eq!(pegs[3].a, Vec2::new(130.0, 150.0));
    }

    #[test]
    fn a_physics_run_spawns_on_schedule_and_is_deterministic() {
        let text = "[video]\nduration = 2\nfps = 30\n[[wall]]\npoints = [[0, 1000], [1080, 1000]]\n[[emitter]]\nat = [540, 100]\ncount = 10\nevery = 0.1\njitter = 30\nradius = [8, 14]\ncolors = [\"#ff5a4e\", \"#4ea3ff\"]\n";
        let run = || {
            let mut r = PhysicsRun::new(PhysicsScene::parse(text).unwrap());
            let mut counts = Vec::new();
            for _ in 0..60 {
                r.advance();
                counts.push(r.spawned);
            }
            (counts, r.world.bodies.iter().map(|b| b.pos.y.to_bits()).collect::<Vec<_>>(), r.world.impacts.len())
        };
        let (counts, ys, impacts) = run();
        assert_eq!(counts[0], 0, "frame 0 is the initial state");
        assert_eq!(*counts.last().unwrap(), 10);
        // 10 spawns at 0.0..0.9 s. The frame at 0.5 s shows the five from
        // 0.0-0.4 s; the 0.5 s spawn enters on the next simulation step.
        assert_eq!(counts[15], 5);
        assert!(impacts > 0);
        assert_eq!(run(), (counts, ys, impacts));
    }

    #[test]
    fn morph_timeline_holds_then_blends_then_holds() {
        let m = MorphScene::parse(
            "[video]\nduration = 4\n[morph]\ncenter = [500, 500]\nscale = 100\npoints = 64\n\
             [[morph.shapes]]\nshape = \"circle\"\ncolor = \"#000000\"\nhold = 1\nmorph = 1\n\
             [[morph.shapes]]\nshape = \"square\"\ncolor = \"#ffffff\"\nhold = 1\n",
        )
        .unwrap();
        let run = MorphRun::new(m).unwrap();
        let (c0, col0) = run.at(0.5);
        let (c_mid, col_mid) = run.at(1.5);
        let (c1, col1) = run.at(3.0);
        assert_eq!(col0, Rgb(0, 0, 0));
        assert_eq!(col1, Rgb(255, 255, 255));
        assert_eq!(col_mid, Rgb(128, 128, 128));
        // Circle points sit at radius 100 from the centre; square corners further out.
        let r = |ring: &Ring| ring.iter().map(|p| (*p - Vec2::new(500.0, 500.0)).len()).fold(0.0, f64::max);
        assert!((r(&c0) - 100.0).abs() < 1e-6);
        assert!(r(&c1) > 115.0);
        assert!(r(&c_mid) > r(&c0) && r(&c_mid) < r(&c1));
    }
}
