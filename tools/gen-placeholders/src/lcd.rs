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

fn round1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
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
    s.local("egg", 26, 22, |c| c.fill(&egg([0., 0.], 0.0), INK));
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
    let small = Text::new(text::BOLD, 13.0).tracking(1.0);
    s.local("am", 26, 16, |c| label(c, &small, "ДП"));
    s.local("pm", 26, 16, |c| label(c, &small, "ПП"));
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

/// An egg outline lying on its side, the way the IM-02 draws them.
fn egg(c: [f32; 2], rot: f32) -> Shape {
    let (rx, ry) = (10.5, 8.0);
    let (s, co) = rot.sin_cos();
    let inside = move |x: f32, y: f32, k: f32| {
        let (dx, dy) = (x - c[0], y - c[1]);
        let u = dx * co + dy * s;
        let v = -dx * s + dy * co;
        // Fatter at one end.
        let ry = (ry - k) * (1.0 + 0.12 * u / rx);
        (u / (rx - k)).powi(2) + (v / ry).powi(2) <= 1.0
    };
    let m = rx + 1.0;
    Shape::new([c[0] - m, c[1] - m, c[0] + m, c[1] + m], move |x, y| {
        inside(x, y, 0.0) && !inside(x, y, 2.8)
    })
}

fn falling(c: &mut Canvas) {
    c.fill(&egg([0., 8.], 1.2), INK);
    // Speed lines.
    for (dx, len) in [(-6.0, 8.0), (0.0, 12.0), (6.0, 8.0)] {
        c.fill(&capsule([dx, -18.0], [dx, -18.0 + len], 2.2), INK);
    }
}

fn wolf(c: &mut Canvas) {
    // Bell-bottoms, knees bent in a jaunty stride.
    c.fill(
        &union_all([
            stroke(&[[292., 290.], [279., 324.], [273., 354.]], 17., 13.),
            stroke(&[[310., 290.], [321., 324.], [331., 354.]], 17., 13.),
            poly(&[[262., 350.], [283., 350.], [287., 369.], [254., 369.]]),
            poly(&[[322., 350.], [341., 350.], [349., 369.], [317., 369.]]),
        ]),
        INK,
    );
    // Big shoes, with a gap under the trouser cuffs.
    c.erase(&rect(240., 368., 120., 2.));
    c.fill(&ellipse([258., 376.], 17., 6.5, 0.0), INK);
    c.fill(&ellipse([342., 376.], 15., 6.5, 0.0), INK);
    // Stars on the trousers.
    for (p, rot) in [
        ([284., 318.], 0.2),
        ([320., 330.], -0.3),
        ([270., 356.], 0.0),
    ] {
        c.erase(&star(p, 4.6, 2.0, 5, rot));
    }

    // Bushy tail.
    let tail = bezier([316., 292.], [346., 296.], [354., 326.], 10);
    c.fill(&stroke(&tail, 11., 6.), INK);
    c.fill(&star([355., 330.], 8., 3.5, 5, 0.4), INK);

    // Hips and torso in a striped sailor's vest.
    c.fill(&ellipse([301., 289.], 24., 13., 0.0), INK);
    c.fill(&ellipse([297., 257.], 22., 32., 0.12), INK);
    for y in [242.0f32, 252., 262., 272.] {
        let band = capsule([268., y], [326., y + 4.], 2.6);
        c.erase(&band.and(ellipse([297., 257.], 17., 27., 0.12)));
    }
    // Belt.
    c.erase(&capsule([278., 283.], [324., 285.], 1.8));

    // Neck and head.
    c.fill(&capsule([289., 222.], [294., 240.], 13.), INK);
    c.fill(&ellipse([285., 200.], 23., 21., 0.0), INK);
    // Long snout, nose on the tip, lower jaw hanging open.
    c.fill(&ellipse([254., 210.], 25., 10., 0.18), INK);
    c.fill(&ellipse([260., 222.], 16., 6., 0.25), INK);
    c.fill(&circle([230., 205.], 6.8), INK);
    c.erase(&circle([228., 203.], 1.8));
    // Grin, and a tooth.
    c.erase(&stroke(
        &bezier([238., 214.], [252., 222.], [272., 218.], 8),
        2.4,
        2.0,
    ));
    c.fill(&poly(&[[246., 216.5], [251., 218.], [248., 222.]]), INK);
    c.erase(&poly(&[[245.5, 217.5], [250., 219.], [248., 221.5]]));
    // Ears laid back, a scruffy forelock and cheek fur.
    c.fill(&poly(&[[288., 185.], [302., 150.], [307., 188.]]), INK);
    c.fill(&poly(&[[273., 184.], [277., 154.], [291., 182.]]), INK);
    c.erase(&poly(&[[293., 180.], [301., 160.], [302., 182.]]));
    c.fill(
        &poly(&[
            [262., 188.],
            [256., 168.],
            [268., 178.],
            [270., 160.],
            [280., 176.],
            [286., 164.],
            [288., 182.],
        ]),
        INK,
    );
    c.fill(
        &poly(&[
            [300., 192.],
            [318., 188.],
            [308., 200.],
            [320., 208.],
            [306., 212.],
            [314., 222.],
            [298., 220.],
        ]),
        INK,
    );
    // Eye with a heavy brow.
    c.erase(&ellipse([270., 195.], 7., 8.5, 0.0));
    c.fill(&circle([267., 197.5], 3.2), INK);
    c.fill(&capsule([261., 187.5], [279., 190.], 4.), INK);
}

/// Arm reaching from the shoulder to a basket at the end of the top or bottom ramp.
fn arm(c: &mut Canvas, top: bool) {
    let (elbow, hand, basket_at) = if top {
        ([248., 238.], [200., 198.], [170., 207.])
    } else {
        ([252., 276.], [200., 291.], [170., 301.])
    };
    c.fill(&stroke(&[SHOULDER, elbow, hand], 11., 8.), INK);
    basket(c, basket_at);
    // The paw gripping the rim.
    c.fill(&ellipse([hand[0] - 2.0, hand[1]], 7., 6., 0.3), INK);
    c.erase(&capsule(
        [hand[0] - 6.0, hand[1] - 2.0],
        [hand[0] - 2.0, hand[1] + 3.0],
        1.2,
    ));
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
    // Lying on his belly on the roof plank, chin up, feet kicked back.
    c.fill(&ellipse([128., 90.], 33., 13., -0.12), INK);
    c.fill(&ellipse([94., 96.], 13., 6., 0.3), INK);
    c.fill(&ellipse([100., 84.], 10., 5., -0.8), INK);
    c.fill(&circle([95., 88.], 5.), INK); // tail puff
    c.erase(&circle([96., 86.], 2.));
    // Head and muzzle.
    c.fill(&ellipse([170., 62.], 17., 15., 0.1), INK);
    c.fill(&ellipse([184., 70.], 10., 7.5, 0.2), INK);
    c.fill(&circle([193., 67.], 3.2), INK);
    // Buck teeth.
    c.fill(&round_rect(180.5, 74., 9., 8.5, 1.5), INK);
    c.erase(&rect(182., 76., 2.6, 5.));
    c.erase(&rect(185.4, 76., 2.6, 5.));
    // Long ears flopping back, with inner ear lines.
    c.fill(&ellipse([150., 32.], 6.5, 23., -0.75), INK);
    c.fill(&ellipse([164., 27.], 6.5, 23., -0.3), INK);
    c.erase(&ellipse([150., 32.], 2.0, 15., -0.75));
    c.erase(&ellipse([164., 27.], 2.0, 15., -0.3));
    // Eye.
    c.erase(&ellipse([175., 57.], 4.5, 5.5, 0.0));
    c.fill(&circle([177., 58.], 2.4), INK);
    // Front paw propping his chin, the other one raised to ring the bell.
    c.fill(
        &stroke(&[[150., 96.], [176., 98.], [184., 88.]], 7., 6.),
        INK,
    );
    c.fill(
        &stroke(&[[156., 86.], [204., 88.], [218., 50.]], 7., 6.),
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
const HEN: Color = hex(0xeeebe0);
const COMB: Color = hex(0xd23a2a);
const BEAK: Color = hex(0xe08a2a);

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
        let g = (hash(x as i32, y as i32) - 0.5) * 0.012;
        let base = mix(hex(0xbcc0b2), hex(0xa9ae9f), t);
        [base[0] + g, base[1] + g, base[2] + g, 1.0]
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
    // The plank under the hare and its brace.
    c.fill(&capsule([78., 106.], [204., 101.], 4.5), PRINT_RED);
    c.fill(&capsule([96., 106.], [80., 82.], 3.), PRINT_RED);
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
    c.img
}
