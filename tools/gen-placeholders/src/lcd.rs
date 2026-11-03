//! The LCD: segment sprites (solid ink, tinted at runtime), the printed background film and
//! the glass overlay.
//!
//! Characters are drawn in LCD coordinates (660×416, origin top-left), so arms, baskets and
//! bodies line up by construction; `Sheet::lcd` crops each one and records where it sits.
//! The right-hand wolf and baskets are mirror images of the left ones around `x = 330`.

use image::RgbaImage;
use serde_json::{Map, Value, json};

use crate::canvas::*;
use crate::text::{self, Text};
use crate::wear::{self, Dice, fbm};

pub const W: u32 = 660;
pub const H: u32 = 416;
const MID: f32 = W as f32 / 2.0;

const INK: Color = hex(0x141614);

/// The sprites to pack and the segment layout.
#[derive(Default)]
pub struct Sheet {
    pub sprites: Vec<(String, RgbaImage)>,
    pub layout: Map<String, Value>,
}

impl Sheet {
    fn place(&mut self, name: &str, sprite: &str, at: [f32; 2]) {
        let at = [round1(at[0]), round1(at[1])];
        self.layout
            .insert(name.into(), json!({ "sprite": sprite, "at": at }));
    }

    /// A sprite drawn around `(0, 0)` inside a `w × h` box, for segments that repeat.
    fn local(&mut self, name: &str, w: u32, h: u32, draw: impl FnOnce(&mut Canvas)) {
        let mut c = Canvas::at(w, h, [-(w as f32) / 2.0, -(h as f32) / 2.0]);
        draw(&mut c);
        self.sprites.push((name.into(), c.img));
    }

    /// A one-off segment drawn in LCD coordinates; it's cropped and placed where it was drawn.
    fn lcd(&mut self, name: &str, draw: impl FnOnce(&mut Canvas)) -> Canvas {
        let mut c = Canvas::new(W, H);
        draw(&mut c);
        let (img, at) = c.crop();
        self.sprites.push((name.into(), img));
        self.place(name, name, at);
        c
    }

    /// Like [`Sheet::lcd`], plus a mirrored twin for the right half of the screen.
    fn lcd_pair(&mut self, left: &str, right: &str, draw: impl FnOnce(&mut Canvas)) {
        let c = self.lcd(left, draw);
        let m = Canvas {
            img: image::imageops::flip_horizontal(&c.img),
            origin: [0.0, 0.0],
        };
        let (img, at) = m.crop();
        self.sprites.push((right.into(), img));
        self.place(right, right, at);
    }
}

/// One decimal, rounded in `f64` so the JSON doesn't print `217.3000030517578`.
fn round1(v: f32) -> f64 {
    (f64::from(v) * 10.0).round() / 10.0
}

// --- where things are ------------------------------------------------------------------

/// Egg positions per ramp, top of the ramp first. Right ramps mirror the left ones.
const EGGS_TL: [[f32; 2]; 5] = [
    [64., 132.],
    [85., 145.],
    [106., 159.],
    [127., 172.],
    [148., 186.],
];
const EGGS_BL: [[f32; 2]; 5] = [
    [58., 228.],
    [80., 241.],
    [103., 254.],
    [125., 267.],
    [148., 280.],
];
const SHOULDER: [f32; 2] = [284., 246.];
const FLOOR: f32 = 388.;
const DIGIT_Y: f32 = 46.;
/// Italic slant of the seven-segment digits (x shift per pixel of height).
const SLANT: f32 = 0.15;

fn mirror(p: [f32; 2]) -> [f32; 2] {
    [2.0 * MID - p[0], p[1]]
}

pub fn segments() -> Sheet {
    let mut s = Sheet::default();

    // Eggs roll down the ramps, then fall when nobody catches them.
    s.local("egg", 30, 26, |c| egg(c, [0., 0.], 0.0));
    for (ramp, pts) in [("tl", EGGS_TL), ("bl", EGGS_BL)] {
        let right = if ramp == "tl" { "tr" } else { "br" };
        for (i, p) in pts.into_iter().enumerate() {
            s.place(&format!("egg_{ramp}_{i}"), "egg", p);
            s.place(&format!("egg_{right}_{i}"), "egg", mirror(p));
        }
    }
    s.local("falling", 28, 44, falling);
    s.place("falling_l", "falling", [150., 324.]);
    s.place("falling_r", "falling", mirror([150., 324.]));

    // The wolf: one body per side, one arm and basket per ramp.
    s.lcd_pair("wolf_l", "wolf_r", wolf);
    s.lcd_pair("arm_tl", "arm_tr", |c| arm(c, true));
    s.lcd_pair("arm_bl", "arm_br", |c| arm(c, false));

    // The floor: a splat, a hatching chick, and the chick running off (0 = nearest).
    s.local("broken", 70, 36, broken);
    s.local("hatched", 40, 38, hatched);
    s.local("chick", 36, 32, |c| chick(c, 1.0));
    s.local("chick_r", 36, 32, |c| {
        let mut m = Canvas::at(36, 32, c.origin);
        chick(&mut m, 1.0);
        c.img = image::imageops::flip_horizontal(&m.img);
    });
    s.place("broken_l", "broken", [166., 376.]);
    s.place("broken_r", "broken", mirror([166., 376.]));
    s.place("hatched_l", "hatched", [164., 371.]);
    s.place("hatched_r", "hatched", mirror([164., 371.]));
    for (i, x) in [116.0f32, 80., 44.].into_iter().enumerate() {
        s.place(&format!("chick_l_{i}"), "chick", [x, 373.]);
        s.place(&format!("chick_r_{i}"), "chick_r", mirror([x, 373.]));
    }

    // The hare lounging on the henhouse roof, and the bell he rings for the alarm.
    s.lcd("hare", hare);
    s.local("bell_0", 30, 34, |c| bell(c, -0.35, true));
    s.local("bell_1", 30, 34, |c| bell(c, 0.35, true));
    // Hanging from his paw by the handle, swinging either way.
    s.place("bell_0", "bell_0", [223.5, 59.]);
    s.place("bell_1", "bell_1", [212.5, 59.]);

    // Misses: chicks under the score.
    s.local("miss", 30, 28, |c| chick(c, 0.8));
    for (i, x) in [390.0f32, 424., 458.].into_iter().enumerate() {
        s.place(&format!("miss_{i}"), "miss", [x, 104.]);
    }

    // Score / clock: a leading "1" (b and c) and three italic seven-segment cells.
    digits(&mut s);

    // Indicators.
    let small = Text::new(text::BOLD, 16.0).tracking(1.0);
    s.local("am", 32, 20, |c| label(c, &small, "ДП"));
    s.local("pm", 32, 20, |c| label(c, &small, "ПП"));
    s.place("am", "am", [318., 30.]);
    s.place("pm", "pm", [318., 50.]);
    s.local("alarm", 24, 26, |c| bell(c, 0.0, false));
    s.place("alarm", "alarm", [318., 72.]);
    let game = Text::new(text::BOLD, 14.0).tracking(1.0);
    s.local("game_a", 66, 16, |c| label(c, &game, "ИГРА А"));
    s.local("game_b", 66, 16, |c| label(c, &game, "ИГРА Б"));
    s.place("game_a", "game_a", [38., 404.]);
    s.place("game_b", "game_b", mirror([38., 404.]));

    s
}

fn label(c: &mut Canvas, t: &Text, s: &str) {
    t.draw(c, s, [0.0, t.cap_height() / 2.0], INK);
}

// --- shapes ----------------------------------------------------------------------------

/// An egg lying on its side: a bold outline around a half-tone body with a glint, so it
/// reads as an egg (not a ring) at a glance.
fn egg(cv: &mut Canvas, c: [f32; 2], rot: f32) {
    const LINE: f32 = 3.2;
    cv.fill(&egg_body(c, rot, 0.0).minus(egg_body(c, rot, LINE)), INK);
    cv.fill(&egg_body(c, rot, LINE), alpha(INK, 0.3));
    let (s, co) = rot.sin_cos();
    let glint = [c[0] - 4.0 * co + 3.0 * s, c[1] - 4.0 * s - 3.0 * co];
    cv.erase(&ellipse(glint, 3.2, 1.8, rot - 0.4));
}

/// The egg's silhouette shrunk by `k`; fatter at one end.
fn egg_body(c: [f32; 2], rot: f32, k: f32) -> Shape {
    let (rx, ry) = (12.0, 9.0);
    let (s, co) = rot.sin_cos();
    let m = rx + 2.0;
    Shape::new([c[0] - m, c[1] - m, c[0] + m, c[1] + m], move |x, y| {
        let (dx, dy) = (x - c[0], y - c[1]);
        let u = dx * co + dy * s;
        let v = -dx * s + dy * co;
        let ry = (ry - k) * (1.0 + 0.12 * u / rx);
        (u / (rx - k)).powi(2) + (v / ry).powi(2) <= 1.0
    })
}

fn falling(c: &mut Canvas) {
    egg(c, [0., 8.], 1.2);
    // Speed lines.
    for (dx, len) in [(-6.0, 8.0), (0.0, 12.0), (6.0, 8.0)] {
        c.fill(&capsule([dx, -18.0], [dx, -18.0 + len], 2.2), INK);
    }
}

/// Our wolf: a lanky, scruffy fellow in dungarees and big shoes, facing left. Solid ink
/// with details cut out in thin light lines, the way LCD segments are printed.
fn wolf(c: &mut Canvas) {
    // Shoes: big rounded clodhoppers, toes pointing the way he faces.
    c.fill(&round_rect(244., 362., 40., 18., 9.), INK);
    c.fill(&round_rect(316., 362., 38., 18., 9.), INK);
    c.erase(&capsule([250., 371.], [262., 371.], 1.4));
    c.erase(&capsule([322., 371.], [334., 371.], 1.4));

    // Legs in dungarees, knees bent a little, wide at the hips.
    c.fill(
        &stroke(&[[290., 296.], [281., 330.], [272., 364.]], 24., 19.),
        INK,
    );
    c.fill(
        &stroke(&[[312., 296.], [322., 330.], [330., 364.]], 24., 19.),
        INK,
    );
    // Turned-up cuffs.
    c.erase(&capsule([262., 352.], [283., 355.], 1.6));
    c.erase(&capsule([320., 355.], [341., 352.], 1.6));

    // Bushy tail sweeping out behind, with notches in the fur.
    let tail = bezier([318., 290.], [356., 286.], [366., 326.], 12);
    c.fill(&stroke(&tail, 16., 9.), INK);
    c.fill(&ellipse([366., 332.], 9., 12., -0.3), INK);
    c.erase(&capsule([364., 340.], [368., 345.], 1.3));
    c.erase(&capsule([350., 300.], [357., 306.], 1.4));
    c.erase(&capsule([359., 314.], [364., 320.], 1.4));

    // Body: one pear shape from the chest down into the legs, so nothing floats.
    c.fill(&ellipse([300., 268.], 27., 40., 0.06), INK);
    c.fill(&ellipse([301., 296.], 28., 16., 0.0), INK);
    // Dungarees: the bib's top edge, straps, buttons, a pocket and the fly seam.
    c.erase(&capsule([280., 258.], [318., 260.], 1.5));
    c.erase(&capsule([282., 258.], [290., 232.], 1.5));
    c.erase(&capsule([316., 260.], [314., 234.], 1.5));
    c.erase(&circle([285., 262.], 2.4));
    c.erase(&circle([314., 264.], 2.4));
    c.erase(&round_frame(292., 268., 15., 12., 3., 1.4));
    c.erase(&capsule([301., 288.], [301., 304.], 1.4));
    // Chest fur poking out above the bib.
    c.erase(&poly(&[
        [292., 240.],
        [296., 250.],
        [300., 242.],
        [304., 251.],
        [308., 241.],
    ]));

    // Neck, then the head, drawn a touch larger than life around the neck (`K`).
    c.fill(&capsule([291., 230.], [295., 242.], 22.), INK);
    const K: f32 = 1.12;
    let t = |p: [f32; 2]| [290. + (p[0] - 290.) * K, 226. + (p[1] - 226.) * K];
    let ts = |pts: &[[f32; 2]]| -> Vec<[f32; 2]> { pts.iter().map(|&p| t(p)).collect() };
    c.fill(&ellipse(t([284., 199.]), 21. * K, 19. * K, -0.1), INK);
    // A little cheek fur at the back of the jaw.
    c.fill(
        &poly(&ts(&[
            [296., 204.],
            [312., 208.],
            [302., 213.],
            [308., 221.],
            [292., 219.],
        ])),
        INK,
    );
    // Long snout with a round nose; the lower jaw hangs open in a grin.
    c.fill(
        &tapered(t([270., 199.]), t([232., 205.]), 22. * K, 14. * K),
        INK,
    );
    c.fill(&circle(t([228., 203.]), 7.5 * K), INK);
    c.erase(&ellipse(t([226., 200.5]), 2.4 * K, 1.6 * K, 0.0));
    c.fill(
        &tapered(t([270., 214.]), t([244., 219.]), 12. * K, 8. * K),
        INK,
    );
    c.erase(&stroke(
        &ts(&bezier([238., 211.], [255., 214.], [270., 208.], 8)),
        3.4 * K,
        2.6 * K,
    ));
    c.fill(
        &poly(&ts(&[[247., 210.], [252., 210.5], [249.5, 216.]])),
        INK,
    );
    c.fill(
        &poly(&ts(&[[259., 210.5], [264., 210.], [261.5, 216.]])),
        INK,
    );
    // Two tall pointed ears with a gap between them and inner lines.
    c.fill(&poly(&ts(&[[272., 186.], [279., 150.], [291., 183.]])), INK);
    c.fill(&poly(&ts(&[[293., 184.], [304., 152.], [306., 192.]])), INK);
    c.erase(&capsule(t([280., 162.]), t([283., 180.]), 1.6 * K));
    c.erase(&capsule(t([302., 164.]), t([301., 182.]), 1.6 * K));
    // A small forelock.
    c.fill(
        &poly(&ts(&[
            [266., 186.],
            [262., 174.],
            [272., 181.],
            [274., 172.],
            [280., 184.],
        ])),
        INK,
    );
    // Big eye glancing toward the ramps, under a heavy brow.
    c.erase(&ellipse(t([266., 192.]), 8. * K, 9. * K, 0.1));
    c.fill(&circle(t([263., 194.]), 3.6 * K), INK);
    c.fill(
        &tapered(t([256., 181.]), t([278., 184.]), 4.5 * K, 3. * K),
        INK,
    );
}

/// Arm reaching from the shoulder to a basket at the end of the top or bottom ramp.
fn arm(c: &mut Canvas, top: bool) {
    let (elbow, hand, basket_at) = if top {
        ([248., 238.], [200., 198.], [170., 207.])
    } else {
        ([252., 276.], [200., 291.], [170., 301.])
    };
    // A furry arm, thick at the shoulder, ending in a round paw.
    c.fill(&stroke(&[SHOULDER, elbow, hand], 15., 10.), INK);
    basket(c, basket_at);
    c.fill(&ellipse([hand[0] - 2.0, hand[1]], 8.5, 7.5, 0.3), INK);
    // Fingers curled over the rim.
    for k in [-1.0f32, 1.0] {
        c.erase(&capsule(
            [hand[0] - 7.0, hand[1] + k * 2.5],
            [hand[0] - 2.5, hand[1] + k * 2.5],
            1.1,
        ));
    }
}

/// A wicker basket: rim, open mouth, woven body, handle.
fn basket(c: &mut Canvas, m: [f32; 2]) {
    let [x, y] = m;
    let body = || {
        poly(&[
            [x - 24., y - 8.],
            [x + 24., y - 8.],
            [x + 17., y + 12.],
            [x - 17., y + 12.],
        ])
        .union(ellipse([x, y + 12.], 17., 4., 0.0))
    };
    c.fill(&body(), INK);
    c.fill(&ellipse([x, y - 8.], 25., 6., 0.0), INK);
    c.erase(&ellipse([x, y - 8.], 20.5, 3.2, 0.0));
    for i in -3..=3 {
        let dx = i as f32 * 7.0;
        let weave = capsule([x + dx - 5., y - 1.], [x + dx + 4., y + 11.], 1.5);
        c.erase(&weave.and(body()));
    }
    c.erase(&capsule([x - 21., y - 1.], [x + 21., y - 1.], 1.3));
    let handle = ring([x, y - 8.], 23., 19., 0.0, 2.6).and(rect(x - 30., y - 40., 60., 31.));
    c.fill(&handle, INK);
}

fn hare(c: &mut Canvas) {
    // Sitting up on the roof plank, one big foot stretched along it, ringing the bell.
    c.fill(&ellipse([140., 88.], 20., 13., 0.0), INK); // haunch
    c.fill(&ellipse([166., 97.5], 15., 4.5, -0.04), INK); // hind foot
    c.erase(&capsule([150., 95.], [176., 94.], 1.2));
    c.fill(&ellipse([152., 70.], 13., 21., 0.25), INK); // body, leaning back
    // Tail puff with a fluffy edge.
    c.fill(&circle([120., 84.], 6.5), INK);
    c.erase(&circle([118.5, 82.5], 3.));
    // Head and muzzle.
    c.fill(&ellipse([168., 46.], 14., 12.5, 0.15), INK);
    c.fill(&ellipse([180., 52.], 8.5, 6.5, 0.2), INK);
    c.fill(&circle([188., 49.], 3.), INK);
    // Buck teeth.
    c.fill(&round_rect(176.5, 56., 8., 7.5, 1.5), INK);
    c.erase(&rect(178., 58., 2.3, 4.5));
    c.erase(&rect(181., 58., 2.3, 4.5));
    // Long ears standing up, tips leaning back, with inner ear lines.
    c.fill(&ellipse([155., 22.], 5.5, 15., -0.35), INK);
    c.fill(&ellipse([167., 19.], 5.5, 15., -0.1), INK);
    c.erase(&ellipse([155., 23.], 1.8, 10., -0.35));
    c.erase(&ellipse([167., 20.], 1.8, 10., -0.1));
    // Eye and a cheeky grin.
    c.erase(&ellipse([172., 42.], 4., 5., 0.0));
    c.fill(&circle([173.5, 43.], 2.2), INK);
    c.erase(&stroke(
        &bezier([174., 55.], [179., 57.], [185., 54.], 5),
        1.4,
        1.2,
    ));
    // One front paw on his knee, the other raised to ring the bell.
    c.fill(
        &stroke(&[[150., 64.], [162., 82.], [172., 86.]], 6.5, 5.5),
        INK,
    );
    c.fill(&ellipse([174., 86.], 4.5, 3.5, 0.0), INK);
    c.fill(
        &stroke(&[[158., 60.], [192., 64.], [218., 50.]], 7., 6.),
        INK,
    );
    c.fill(&circle([218., 47.], 5.), INK);
}

/// A hand bell, tilted by `tilt`; `ringing` adds the little ring marks.
fn bell(c: &mut Canvas, tilt: f32, ringing: bool) {
    let (s, co) = tilt.sin_cos();
    let rot = |p: [f32; 2]| [p[0] * co - p[1] * s, p[0] * s + p[1] * co + 2.0];
    let dome = poly(&[
        rot([-4., -9.]),
        rot([4., -9.]),
        rot([8., 2.]),
        rot([11., 7.]),
        rot([-11., 7.]),
        rot([-8., 2.]),
    ])
    .union(circle(rot([0., -9.]), 4.0));
    c.fill(&dome, INK);
    c.fill(&capsule(rot([0., -13.]), rot([0., -16.]), 3.), INK);
    c.fill(&circle(rot([0., 10.]), 2.6), INK);
    if ringing {
        let side = if tilt < 0.0 { 1.0 } else { -1.0 };
        for (dy, len) in [(-6.0, 4.0), (0.0, 5.0)] {
            let x = side * 12.0;
            c.fill(&capsule([x, dy], [x + side * len, dy - 2.0], 1.6), INK);
        }
    }
}

fn broken(c: &mut Canvas) {
    // A starburst splat, its yolk, and two bits of shell flung aside.
    c.fill(&star([0., 2.], 16., 7.5, 8, 0.2), INK);
    c.erase(&star([0., 2.], 11., 5., 8, 0.2));
    c.fill(&circle([0., 3.], 4.), INK);
    for (x, flip) in [(-26.0, 1.0), (26.0, -1.0)] {
        let shell = ellipse([x, 10.], 7., 5., 0.0)
            .minus(ellipse([x, 7.], 6., 4., 0.0))
            .and(rect(x - 8., 6., 16., 12.));
        c.fill(&shell, INK);
        c.fill(
            &capsule([x + flip * 7., 9.], [x + flip * 10., 6.], 1.6),
            INK,
        );
    }
}

fn hatched(c: &mut Canvas) {
    // Chick head poking out of a cracked shell.
    c.fill(&circle([0., -6.], 8.5), INK);
    c.fill(&poly(&[[7., -8.], [14., -6.], [7., -3.]]), INK);
    c.erase(&circle([3., -8.5], 1.7));
    // Fluff on top.
    c.fill(&capsule([-2., -14.], [-4., -18.], 2.), INK);
    c.fill(&capsule([1., -14.], [2., -18.], 2.), INK);
    let shell = ellipse([0., 8.], 14., 11., 0.0).and(rect(-15., 2., 30., 18.));
    let zig = poly(&[
        [-16., 2.],
        [-10., 7.],
        [-5., 1.],
        [0., 7.],
        [5., 1.],
        [10., 7.],
        [16., 2.],
        [16., -2.],
        [-16., -2.],
    ]);
    c.fill(&shell.minus(zig), INK);
    c.erase(&ellipse([-6., 12.], 2.2, 3.5, 0.4));
}

/// A chick running to the left, scaled by `k`.
fn chick(c: &mut Canvas, k: f32) {
    let p = |x: f32, y: f32| [x * k, y * k];
    c.fill(&ellipse(p(3., 3.), 10. * k, 8. * k, -0.15), INK);
    c.fill(&circle(p(-6., -6.), 6.5 * k), INK);
    c.fill(&poly(&[p(-11., -8.), p(-17., -5.), p(-11., -3.5)]), INK);
    c.fill(&poly(&[p(10., -1.), p(16., -6.), p(14., 3.)]), INK);
    c.erase(&circle(p(-7.5, -7.5), 1.7 * k));
    c.erase(&stroke(
        &bezier(p(-1., 2.), p(5., 7.), p(10., 1.), 6),
        1.6 * k,
        1.6 * k,
    ));
    // Legs mid-stride.
    c.fill(
        &stroke(&[p(0., 10.), p(-5., 14.), p(-9., 14.)], 2.2 * k, 2.0 * k),
        INK,
    );
    c.fill(
        &stroke(&[p(6., 10.), p(9., 14.), p(13., 13.)], 2.2 * k, 2.0 * k),
        INK,
    );
}

fn digits(s: &mut Sheet) {
    let shear = |pts: &[[f32; 2]]| -> Vec<[f32; 2]> {
        pts.iter().map(|&[x, y]| [x - y * SLANT, y]).collect()
    };
    let horiz = [
        [-10.5, 0.],
        [-7.5, -3.],
        [7.5, -3.],
        [10.5, 0.],
        [7.5, 3.],
        [-7.5, 3.],
    ];
    let vert = [
        [0., -11.5],
        [3., -8.5],
        [3., 8.5],
        [0., 11.5],
        [-3., 8.5],
        [-3., -8.5],
    ];
    for seg in ["a", "d", "g"] {
        s.local(&format!("seg_{seg}"), 24, 10, |c| {
            c.fill(&poly(&horiz), INK)
        });
    }
    for seg in ["b", "c", "e", "f"] {
        s.local(&format!("seg_{seg}"), 12, 26, |c| {
            c.fill(&poly(&shear(&vert)), INK)
        });
    }
    let at = |cx: f32, dx: f32, dy: f32| [cx + dx - dy * SLANT, DIGIT_Y + dy];
    const SEGS: [(&str, f32, f32); 7] = [
        ("a", 0., -25.),
        ("b", 12.5, -12.5),
        ("c", 12.5, 12.5),
        ("d", 0., 25.),
        ("e", -12.5, 12.5),
        ("f", -12.5, -12.5),
        ("g", 0., 0.),
    ];
    let one = 338.0;
    s.place("one_b", "seg_b", at(one, 12.5, -12.5));
    s.place("one_c", "seg_c", at(one, 12.5, 12.5));
    for (i, cx) in [376.0f32, 432., 476.].into_iter().enumerate() {
        for (seg, dx, dy) in SEGS {
            s.place(
                &format!("d{i}_{seg}"),
                &format!("seg_{seg}"),
                at(cx, dx, dy),
            );
        }
    }
    s.local("colon", 12, 30, |c| {
        for y in [-9.0f32, 9.0] {
            c.fill(&circle([-y * SLANT, y], 3.0), INK);
        }
    });
    s.place("colon", "colon", [404., DIGIT_Y]);
}

// --- the printed background ----------------------------------------------------------

const PRINT_RED: Color = hex(0xb4452e);
const PRINT_GREEN: Color = hex(0x2e8c62);
const PRINT_GREEN_DARK: Color = hex(0x1d5e40);
const PRINT_INK: Color = hex(0x262824);
const STRAW: Color = hex(0xc6a050);
// The hens are printed in the film's own inks so they sit with the henhouses.
const HEN: Color = hex(0xdedccd);
const COMB: Color = PRINT_RED;
const BEAK: Color = STRAW;

/// Draw `f` on a scratch canvas, then copy it mirrored onto the right half too.
fn both_sides(c: &mut Canvas, f: impl Fn(&mut Canvas)) {
    let mut l = Canvas::new(W, H);
    f(&mut l);
    c.draw(&l, 1.0);
    let r = Canvas {
        img: image::imageops::flip_horizontal(&l.img),
        origin: [0.0, 0.0],
    };
    c.draw(&r, 1.0);
}

pub fn background() -> RgbaImage {
    let mut c = Canvas::new(W, H);
    // Reflective LCD: a cool grey-green with a faint vertical falloff and grain.
    c.paint(&rect(0., 0., W as f32, H as f32), |x, y| {
        let t = y / H as f32;
        let g = (grain(x, y) - 0.5) * 0.012;
        let base = mix(hex(0xbcc0b2), hex(0xa9ae9f), t);
        [base[0] + g, base[1] + g, base[2] + g, 1.0]
    });

    // An old LCD: the polarizer has gone a little brown in one corner and the contrast is
    // uneven across the film.
    c.paint(&rect(0., 0., W as f32, H as f32), |x, y| {
        let d = ((x - W as f32).powi(2) + (y - H as f32).powi(2)).sqrt() / 260.0;
        let corner = (1.0 - d).max(0.0).powi(2) * 0.16;
        let blotch = fbm(x, y, 90.0) * 0.05;
        [0.42, 0.33, 0.16, corner + blotch]
    });

    // Henhouses: posts, perches and ramps on both sides.
    both_sides(&mut c, |c| {
        c.fill(&rect(0., 36., 5., 290.), PRINT_RED);
        for (y, w) in [(112.0f32, 58.0f32), (208., 52.)] {
            c.fill(&rect(0., y, w, 5.), PRINT_RED);
            c.fill(
                &capsule([w - 4., y + 4.], [w - 14., y + 20.], 3.),
                PRINT_RED,
            );
        }
        ramp(c, &EGGS_TL, false);
        ramp(c, &EGGS_BL, true);
        for (center, flip) in [([30., 96.], false), ([27., 192.], false)] {
            hen(c, center, flip);
        }
        // A bush at the foot of each henhouse.
        bush(
            c,
            &[([18., 352.], 20.), ([44., 358.], 16.), ([-2., 340.], 18.)],
        );
        // Grass tufts and the ground line.
        for x in [214.0f32, 238., 262.] {
            tuft(c, [x, FLOOR - 2.0]);
        }
        for x in (0..330).step_by(22) {
            c.fill(
                &capsule([x as f32 + 4., FLOOR], [x as f32 + 14., FLOOR], 1.6),
                alpha(PRINT_INK, 0.55),
            );
        }
    });

    // Top left: the henhouse roof the hare lounges on.
    let roof = |dy: f32| capsule([0., 14. + dy], [86., 74. + dy], 5.);
    c.fill(&roof(0.0), PRINT_RED);
    c.fill(&roof(14.0), PRINT_RED);
    for i in 0..6 {
        let x = 8.0 + i as f32 * 14.0;
        let y = 14.0 + x * 60.0 / 86.0;
        c.fill(&capsule([x, y], [x - 4.0, y + 14.0], 2.4), PRINT_RED);
    }
    // The plank under the hare: a board nailed to the roof, held up by a knee brace.
    c.fill(&capsule([80., 106.], [206., 101.], 4.5), PRINT_RED);
    c.fill(&capsule([84., 113.], [190., 109.], 2.6), PRINT_RED);
    for x in [110.0f32, 140., 170.] {
        let y = 106.0 - (x - 80.0) * 5.0 / 126.0;
        c.fill(&capsule([x, y], [x, y + 6.5], 2.2), PRINT_RED);
    }
    c.fill(&capsule([84., 113.], [80., 80.], 4.), PRINT_RED);
    c.fill(&capsule([118., 110.], [82., 88.], 3.), PRINT_RED);
    // Top right: a tree.
    bush(
        &mut c,
        &[
            ([628., 12.], 22.),
            ([652., 26.], 18.),
            ([604., 28.], 16.),
            ([640., 44.], 14.),
            ([660., 2.], 18.),
        ],
    );

    c.img
}

/// Two rails under the egg path, with legs, or a fence underneath for the lower ramps.
fn ramp(c: &mut Canvas, eggs: &[[f32; 2]; 5], fence: bool) {
    let (a, b) = (eggs[0], eggs[4]);
    let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let d = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
    let n = [-d[1], d[0]];
    let off = |p: [f32; 2], along: f32, out: f32| {
        [
            p[0] + d[0] * along + n[0] * out,
            p[1] + d[1] * along + n[1] * out,
        ]
    };
    let (start, end) = (off(a, -20.0, 0.0), off(b, 12.0, 0.0));
    c.fill(
        &capsule(off(start, 0., 10.), off(end, 0., 10.), 4.5),
        PRINT_RED,
    );
    c.fill(
        &capsule(off(start, 0., 17.), off(end, 0., 17.), 2.6),
        PRINT_RED,
    );
    if fence {
        for i in 0..7 {
            let p = off(start, 8.0 + i as f32 * 16.0, 17.0);
            c.fill(&capsule(p, [p[0], p[1] + 22.0], 3.0), PRINT_RED);
        }
        let p = off(start, 0.0, 17.0);
        let q = off(end, -6.0, 17.0);
        c.fill(
            &capsule([p[0], p[1] + 22.0], [q[0], q[1] + 22.0], 3.0),
            PRINT_RED,
        );
    } else {
        for t in [0.3f32, 0.85] {
            let p = off(start, t * (len + 32.0), 17.0);
            c.fill(&capsule(p, [p[0] - 3.0, p[1] + 14.0], 2.6), PRINT_RED);
        }
    }
}

/// A broody hen on a straw nest, in black outline on a white body; faces right.
fn hen(c: &mut Canvas, at: [f32; 2], _flip: bool) {
    let [x, y] = at;
    // Nest.
    c.fill(&ellipse([x, y + 12.], 22., 6., 0.0), STRAW);
    for i in -4..=4 {
        let sx = x + i as f32 * 5.0;
        c.fill(
            &capsule([sx - 3., y + 9.], [sx + 3., y + 15.], 1.2),
            alpha(hex(0x8a6a2a), 0.8),
        );
    }
    // Tail feathers.
    for (dx, dy) in [(-24.0, -10.0), (-26.0, -3.0), (-23.0, 3.0)] {
        c.fill(&capsule([x - 12., y], [x + dx, y + dy], 6.), PRINT_INK);
        c.fill(&capsule([x - 12., y], [x + dx, y + dy], 3.), HEN);
    }
    // Body and head, outlined.
    c.fill(&ellipse([x, y], 18., 13., 0.0), PRINT_INK);
    c.fill(&ellipse([x, y], 15.5, 10.5, 0.0), HEN);
    c.fill(&circle([x + 13., y - 13.], 8.), PRINT_INK);
    c.fill(&circle([x + 13., y - 13.], 5.6), HEN);
    c.fill(&ellipse([x + 7., y - 6.], 4.5, 5.5, 0.4), HEN);
    // Comb, wattle, beak, eye.
    for (dx, r) in [(-3.0, 3.2), (1.0, 3.8), (5.0, 3.0)] {
        c.fill(&circle([x + 12. + dx, y - 22.], r), COMB);
    }
    c.fill(&ellipse([x + 19., y - 6.], 2.5, 4., 0.0), COMB);
    c.fill(
        &poly(&[[x + 20., y - 16.], [x + 27., y - 13.], [x + 20., y - 10.]]),
        BEAK,
    );
    c.fill(&circle([x + 15., y - 15.], 1.6), PRINT_INK);
    // Wing.
    let wing = bezier([x - 9., y - 3.], [x + 1., y + 8.], [x + 10., y - 1.], 8);
    c.fill(&stroke(&wing, 2.2, 1.4), PRINT_INK);
}

fn bush(c: &mut Canvas, blobs: &[([f32; 2], f32)]) {
    let shape = || union_all(blobs.iter().map(|&(p, r)| circle(p, r)));
    c.fill(&shape(), PRINT_GREEN_DARK);
    let inner = union_all(
        blobs
            .iter()
            .map(|&(p, r)| circle([p[0], p[1] + 1.5], r - 2.5)),
    );
    c.fill(&inner, PRINT_GREEN);
    // Leaf marks.
    for &(p, r) in blobs {
        let q = [p[0] - r * 0.3, p[1] - r * 0.25];
        let leaf = bezier(q, [q[0] + 4., q[1] - 3.], [q[0] + 8., q[1]], 5);
        c.fill(&stroke(&leaf, 2.0, 1.2), PRINT_GREEN_DARK);
    }
}

fn tuft(c: &mut Canvas, at: [f32; 2]) {
    let [x, y] = at;
    for (dx, h, lean) in [
        (-6.0, 9.0, -3.0),
        (-2.0, 13.0, -1.0),
        (2.0, 11.0, 2.0),
        (6.0, 8.0, 4.0),
    ] {
        c.fill(
            &tapered([x + dx, y], [x + dx + lean, y - h], 3.0, 0.8),
            PRINT_GREEN,
        );
    }
}

// --- the glass ---------------------------------------------------------------------------

/// Drawn over the segments: the frame's shadow on the glass and a soft glare.
pub fn glass() -> RgbaImage {
    let (w, h) = (W as f32, H as f32);
    let mut c = Canvas::new(W, H);
    let all = || rect(0., 0., w, h);
    c.inner_shadow(&all(), [4., 5.], 12., [0.0, 0.0, 0.0, 0.5]);
    c.inner_shadow(&all(), [-1.5, -1.5], 2., [1.0, 1.0, 1.0, 0.25]);
    c.paint(&all(), |x, y| {
        // A broad diagonal glare band, strongest top-left.
        let d = (x + y * 1.4 - 230.0) / 120.0;
        let band = (-d * d).exp() * 0.10;
        let fall = 1.0 - (x + y) / (w + h);
        [1.0, 1.0, 1.0, (band + 0.04 * fall) * fall.max(0.3)]
    });
    // The window is scuffed: hairline scratches, dust caught under the frame, and a
    // fingerprint smudge near the lower right where thumbs come off the buttons.
    let mut dice = Dice::new(0x02);
    let all_in = |_: f32, _: f32| true;
    wear::scratches(
        &mut c,
        &mut dice,
        [0., 0., w, h],
        140,
        [6., 60.],
        0.85,
        &all_in,
    );
    wear::dust(&mut c, &mut dice, [0., 0., w, h], 160);
    for (x, y) in [
        (6.0f32, 6.0f32),
        (w - 6.0, 8.0),
        (8.0, h - 6.0),
        (w - 8.0, h - 6.0),
    ] {
        wear::dust(&mut c, &mut dice, [x - 6., y - 6., x + 6., y + 6.], 40);
    }
    let print_at = [w - 120., h - 90.];
    c.paint(&ellipse(print_at, 46., 34., 0.5), move |x, y| {
        let (dx, dy) = (x - print_at[0], y - print_at[1]);
        let r = (dx * dx / 1.4 + dy * dy).sqrt();
        let ridge = (r * 1.6 + fbm(x, y, 6.0) * 9.0).sin() * 0.5 + 0.5;
        let fade = (1.0 - r / 40.0).max(0.0) * fbm(x + 50.0, y, 14.0);
        [1., 1., 1., 0.045 * ridge * fade]
    });
    c.img
}
