//! Wear and tear: the device looks like it spent forty years in a coat pocket. Everything is
//! seeded, so regenerating the art gives the same scratches every time.

use crate::canvas::*;

/// A small deterministic xorshift generator.
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform in `0.0..1.0`.
    pub fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

/// Smooth value noise in `0.0..1.0`, one cell per drawing unit.
fn value(x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0, y - y0);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let h = |dx: i32, dy: i32| hash(x0 as i32 + dx, y0 as i32 + dy);
    let a = h(0, 0) + (h(1, 0) - h(0, 0)) * sx;
    let b = h(0, 1) + (h(1, 1) - h(0, 1)) * sx;
    a + (b - a) * sy
}

/// Fractal noise in `0.0..1.0` with features about `size` drawing units across.
pub fn fbm(x: f32, y: f32, size: f32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0 / size, 0.0);
    for i in 0..4 {
        let o = i as f32 * 17.3;
        sum += amp * value(x * f + o, y * f - o);
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    sum / norm
}

/// Fine scratches inside `bbox`, each a slightly curved hairline. `light` scratches catch
/// the light; the rest are faint dark grooves with a highlight on their lower lip.
pub fn scratches(
    c: &mut Canvas,
    dice: &mut Dice,
    bbox: [f32; 4],
    n: usize,
    len: [f32; 2],
    light: f32,
    inside: &dyn Fn(f32, f32) -> bool,
) {
    for _ in 0..n {
        let p0 = [dice.range(bbox[0], bbox[2]), dice.range(bbox[1], bbox[3])];
        if !inside(p0[0], p0[1]) {
            continue;
        }
        // Mostly one direction (how it slid in and out of a pocket), some random.
        let a = if dice.f() < 0.6 {
            dice.range(-0.35, 0.15)
        } else {
            dice.range(0.0, std::f32::consts::TAU)
        };
        let l = dice.range(len[0], len[1]);
        let p1 = [p0[0] + a.cos() * l, p0[1] + a.sin() * l];
        let bend = dice.range(-0.12, 0.12) * l;
        let mid = [
            (p0[0] + p1[0]) / 2.0 - a.sin() * bend,
            (p0[1] + p1[1]) / 2.0 + a.cos() * bend,
        ];
        let pts = bezier(p0, mid, p1, 6);
        let w = dice.range(0.35, 0.8);
        let strength = dice.range(0.4, 1.0);
        if dice.f() < light {
            c.fill(&stroke(&pts, w, w * 0.4), [1., 1., 1., 0.22 * strength]);
        } else {
            c.fill(
                &stroke(&pts, w, w * 0.4),
                [0.1, 0.09, 0.07, 0.12 * strength],
            );
            let lip: Vec<[f32; 2]> = pts.iter().map(|p| [p[0], p[1] + 0.6]).collect();
            c.fill(
                &stroke(&lip, w * 0.6, w * 0.3),
                [1., 1., 1., 0.1 * strength],
            );
        }
    }
}

/// Dust specks and lint: tiny pale and dark dots.
pub fn dust(c: &mut Canvas, dice: &mut Dice, bbox: [f32; 4], n: usize) {
    for _ in 0..n {
        let p = [dice.range(bbox[0], bbox[2]), dice.range(bbox[1], bbox[3])];
        let r = dice.range(0.3, 1.1);
        let col = if dice.f() < 0.7 {
            [1., 1., 0.95, dice.range(0.15, 0.4)]
        } else {
            [0.2, 0.18, 0.14, dice.range(0.1, 0.3)]
        };
        c.fill(&circle(p, r), col);
    }
}

/// Grime packed into a crevice: `s` (usually a thin frame or ring) filled with blotchy
/// brown-grey dirt, softened so it sits in the corner rather than on top of it.
pub fn grime(c: &mut Canvas, s: &Shape, strength: f32) {
    let (w, h) = c.size();
    let mut dirt = Canvas::at(w, h, c.origin);
    dirt.paint(s, |x, y| {
        let n = fbm(x, y, 16.0);
        let t = ((n - 0.25) / 0.45).clamp(0.0, 1.0);
        [0.26, 0.22, 0.15, strength * (0.35 + 0.65 * t)]
    });
    dirt.blur(1.2);
    c.draw(&dirt, 1.0);
}

/// Rub off printed ink: alpha is knocked back where the noise is high, more so where
/// `rub(x, y)` (0..1) says fingers touch it often.
pub fn erode(c: &mut Canvas, seed: f32, rub: impl Fn(f32, f32) -> f32) {
    c.fade(|x, y| {
        let coarse = fbm(x + seed, y - seed, 9.0);
        let fine = grain(x + seed, y);
        let wear = 0.18 + 0.55 * rub(x, y);
        let t = ((coarse - (1.0 - wear)) / 0.18).clamp(0.0, 1.0);
        // Pitting: tiny holes everywhere, more on the worn patches.
        let pit = if fine < 0.04 + 0.12 * t { 0.35 } else { 1.0 };
        (1.0 - 0.8 * t) * pit
    });
}
