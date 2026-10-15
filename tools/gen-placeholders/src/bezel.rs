//! The IM-02 casing: grey moulded shell, a recessed silver panel around the LCD with the
//! printed title and model line, the "ИМ" badge, red push buttons in pale sockets, and the
//! pill buttons with the bell/clock marks and the recessed set buttons.

use image::RgbaImage;
use serde_json::{Value, json};

use crate::canvas::*;
use crate::lcd;
use crate::text::{self, Text};

pub const W: u32 = 1120;
pub const H: u32 = 672;
pub const LCD_ORIGIN: [f32; 2] = [230., 86.];

const PRINT: Color = hex(0x1a1a1a);
const TITLE_RED: Color = hex(0xd8342a);
const BUTTON_RED: Color = hex(0xd9362c);
const BUTTON_RED_DARK: Color = hex(0x8e1812);
const BUTTON_RED_LIGHT: Color = hex(0xff7a66);

/// Big ramp buttons: (name, center, arrow direction).
const BIG: [(&str, [f32; 2], [f32; 2]); 4] = [
    ("ramp_tl", [92., 404.], [-1., -1.]),
    ("ramp_bl", [92., 534.], [-1., 1.]),
    ("ramp_tr", [1028., 404.], [1., -1.]),
    ("ramp_br", [1028., 534.], [1., 1.]),
];
const SOCKET_R: f32 = 44.;
const DOME_R: f32 = 31.;
/// Pill buttons: (name, center y, label).
const PILLS: [(&str, f32, &str); 3] = [
    ("game_a", 74., "игра А"),
    ("game_b", 144., "игра Б"),
    ("time", 214., "время"),
];
const PILL_X: f32 = 990.;
const PILL: [f32; 2] = [64., 30.];
const ICON_X: f32 = 1072.;
/// Recessed pin buttons: (name, center y).
const HOLES: [(&str, f32); 2] = [("alarm_set", 100.), ("time_set", 176.)];
const HOLE_R: f32 = 8.;

/// Button hit shapes for `layout.json`.
pub fn buttons() -> Value {
    let mut m = serde_json::Map::new();
    for (name, [x, y], _) in BIG {
        m.insert(name.into(), json!({ "circle": [x, y, SOCKET_R] }));
    }
    for (name, y, _) in PILLS {
        let r = [PILL_X - PILL[0] / 2., y - PILL[1] / 2., PILL[0], PILL[1]];
        m.insert(name.into(), json!({ "rect": r }));
    }
    for (name, y) in HOLES {
        m.insert(name.into(), json!({ "circle": [ICON_X, y, HOLE_R + 4.] }));
    }
    Value::Object(m)
}

fn vgrad(top: Color, bottom: Color, y0: f32, y1: f32) -> impl Fn(f32, f32) -> Color {
    move |_, y| mix(top, bottom, (y - y0) / (y1 - y0))
}

pub fn bezel() -> RgbaImage {
    let (w, h) = (W as f32, H as f32);
    let mut c = Canvas::new(W, H);

    // Moulded shell with a soft contact shadow.
    let shell = || round_rect(8., 6., w - 16., h - 14., 40.);
    c.shadow(&shell(), [0., 3.], 5., [0., 0., 0., 0.5]);
    c.paint(&shell(), |x, y| {
        let base = mix(hex(0xd6d7d2), hex(0xbdbeb8), y / h);
        let g = (hash(x as i32, y as i32) - 0.5) * 0.015;
        [base[0] + g, base[1] + g, base[2] + g, 1.0]
    });
    c.inner_shadow(&shell(), [0., 2.5], 3., [1., 1., 1., 0.7]);
    c.inner_shadow(&shell(), [0., -3.], 4., [0., 0., 0., 0.35]);
    // The raised rim step and the groove above the bottom lip.
    c.fill(
        &round_frame(22., 18., w - 44., h - 40., 30., 1.6),
        [0., 0., 0., 0.12],
    );
    c.fill(
        &round_frame(23., 19.5, w - 46., h - 42., 29., 1.2),
        [1., 1., 1., 0.45],
    );
    c.fill(
        &capsule([40., 616.], [w - 40., 616.], 2.),
        [0., 0., 0., 0.16],
    );
    c.fill(
        &capsule([40., 618.5], [w - 40., 618.5], 1.4),
        [1., 1., 1., 0.5],
    );

    // The recessed silver panel: a chamfer, then the brushed face.
    let chamfer = || round_rect(172., 24., 776., 570., 26.);
    c.paint(&chamfer(), vgrad(hex(0xa9aaa5), hex(0xe2e3df), 24., 594.));
    c.fill(
        &round_frame(172., 24., 776., 570., 26., 1.2),
        [0., 0., 0., 0.2],
    );
    let face = || round_rect(182., 34., 756., 550., 20.);
    c.paint(&face(), |x, y| {
        let base = mix(hex(0xdcddd9), hex(0xc8c9c4), (y - 34.) / 550.);
        // Brushed: fine horizontal streaks.
        let streak =
            (hash(0, y as i32) - 0.5) * 0.03 + (hash(x as i32 / 37, y as i32) - 0.5) * 0.012;
        [base[0] + streak, base[1] + streak, base[2] + streak, 1.0]
    });
    c.inner_shadow(&face(), [0., 3.], 5., [0., 0., 0., 0.22]);
    c.inner_shadow(&face(), [0., -1.5], 2., [1., 1., 1., 0.5]);

    // Grey surround and the dark lip right around the glass.
    let [lx, ly] = LCD_ORIGIN;
    let (lw, lh) = (lcd::W as f32, lcd::H as f32);
    let surround = || round_rect(lx - 16., ly - 16., lw + 32., lh + 32., 12.);
    c.shadow(&surround(), [0., 1.5], 2., [1., 1., 1., 0.6]);
    c.paint(
        &surround(),
        vgrad(hex(0xb0b1ac), hex(0xc6c7c2), ly - 16., ly + lh + 16.),
    );
    c.inner_shadow(&surround(), [0., 2.], 3., [0., 0., 0., 0.25]);
    c.fill(
        &round_rect(lx - 4., ly - 4., lw + 8., lh + 8., 4.),
        hex(0x2c2d2a),
    );

    // The printed frame line, broken by the title and the model name.
    let title = Text::new(text::BOLD, 31.0).tracking(3.0);
    let model = Text::new(text::BOLD, 27.0).tracking(4.0);
    let (top, bottom) = (54.0, 552.0);
    let mut line = Canvas::new(W, H);
    line.fill(
        &round_frame(200., top - 1.75, 720., bottom - top + 3.5, 16., 3.5),
        PRINT,
    );
    let gap = |t: &Text, s: &str, y: f32| {
        let half = t.width(s) / 2.0 + 14.0;
        rect(w / 2.0 - half, y - 20.0, half * 2.0, 40.0)
    };
    line.erase(&gap(&title, "НУ ,ПОГОДИ!", top));
    line.erase(&gap(&model, "ЭЛЕКТРОНИКА ИМ 02", bottom));
    c.draw(&line, 1.0);
    let baseline = |t: &Text, y: f32| y + t.cap_height() / 2.0;
    title.draw(
        &mut c,
        "НУ ,ПОГОДИ!",
        [w / 2.0, baseline(&title, top)],
        TITLE_RED,
    );
    model.draw(
        &mut c,
        "ЭЛЕКТРОНИКА ИМ 02",
        [w / 2.0, baseline(&model, bottom)],
        PRINT,
    );

    // The "ИМ" badge.
    let badge = [94., 98.];
    c.fill(&ring(badge, 40., 40., 0.0, 3.4), PRINT);
    c.fill(&circle([badge[0] - 40., badge[1]], 6.), PRINT);
    let im = Text::new(text::CONDENSED, 40.0);
    im.draw(
        &mut c,
        "ИМ",
        [badge[0] + 2., baseline(&im, badge[1])],
        PRINT,
    );

    for (_, at, dir) in BIG {
        big_button(&mut c, at);
        arrow(&mut c, at, dir);
    }

    let label = Text::new(text::BOLD, 17.0);
    for (_, y, s) in PILLS {
        pill_button(&mut c, [PILL_X, y]);
        label.draw(&mut c, s, [PILL_X, y + 36.], PRINT);
    }
    bell_icon(&mut c, [ICON_X, 68.]);
    clock_icon(&mut c, [ICON_X, 142.]);
    for (_, y) in HOLES {
        hole(&mut c, [ICON_X, y]);
    }

    c.img
}

fn big_button(c: &mut Canvas, at: [f32; 2]) {
    // Pale raised socket.
    c.shadow(&circle(at, SOCKET_R), [0., 2.], 3., [0., 0., 0., 0.3]);
    c.paint(
        &circle(at, SOCKET_R),
        vgrad(
            hex(0xeeeeea),
            hex(0xc4c5c0),
            at[1] - SOCKET_R,
            at[1] + SOCKET_R,
        ),
    );
    // The well the dome sits in.
    let well = || circle(at, DOME_R + 6.);
    c.fill(&well(), hex(0x9a9b96));
    c.inner_shadow(&well(), [0., 3.], 5., [0., 0., 0., 0.55]);
    dome(c, at, DOME_R);
}

/// A glossy red dome lit from the top left.
fn dome(c: &mut Canvas, at: [f32; 2], r: f32) {
    c.shadow(&circle(at, r), [0., r * 0.13], r * 0.15, [0., 0., 0., 0.5]);
    let light = [at[0] - r * 0.35, at[1] - r * 0.4];
    c.paint(&circle(at, r), move |x, y| {
        let d = ((x - light[0]).powi(2) + (y - light[1]).powi(2)).sqrt() / (r * 1.6);
        if d < 0.35 {
            mix(BUTTON_RED_LIGHT, BUTTON_RED, d / 0.35)
        } else {
            mix(BUTTON_RED, BUTTON_RED_DARK, (d - 0.35) / 0.65)
        }
    });
    c.inner_shadow(
        &circle(at, r),
        [0., -r * 0.08],
        r * 0.12,
        [0.25, 0., 0., 0.4],
    );
    let spec = [at[0] - r * 0.32, at[1] - r * 0.45];
    c.paint(&ellipse(spec, r * 0.38, r * 0.2, -0.5), move |x, y| {
        let d = ((x - spec[0]) / (r * 0.38)).powi(2) + ((y - spec[1]) / (r * 0.2)).powi(2);
        [1., 1., 1., 0.55 * (1.0 - d).max(0.0)]
    });
}

/// Black arrowhead pointing from the ramp's corner toward its button.
fn arrow(c: &mut Canvas, at: [f32; 2], dir: [f32; 2]) {
    let n = std::f32::consts::FRAC_1_SQRT_2;
    let d = [dir[0] * n, dir[1] * n];
    let base = [at[0] + d[0] * 70., at[1] + d[1] * 70.];
    let tip = [at[0] + d[0] * 50., at[1] + d[1] * 50.];
    let p = [-d[1], d[0]];
    c.fill(
        &poly(&[
            tip,
            [base[0] + p[0] * 10., base[1] + p[1] * 10.],
            [base[0] - p[0] * 10., base[1] - p[1] * 10.],
        ]),
        PRINT,
    );
}

fn pill_button(c: &mut Canvas, at: [f32; 2]) {
    let [x, y] = at;
    let [pw, ph] = PILL;
    let socket = || {
        round_rect(
            x - pw / 2. - 7.,
            y - ph / 2. - 7.,
            pw + 14.,
            ph + 14.,
            ph / 2. + 7.,
        )
    };
    c.shadow(&socket(), [0., 1.5], 2., [0., 0., 0., 0.3]);
    c.paint(
        &socket(),
        vgrad(hex(0xefefeb), hex(0xc4c5c0), y - ph, y + ph),
    );
    let well = || {
        round_rect(
            x - pw / 2. - 3.,
            y - ph / 2. - 3.,
            pw + 6.,
            ph + 6.,
            ph / 2. + 3.,
        )
    };
    c.fill(&well(), hex(0x8e8f8a));
    c.inner_shadow(&well(), [0., 2.], 3., [0., 0., 0., 0.5]);
    let cap = || round_rect(x - pw / 2., y - ph / 2., pw, ph, ph / 2.);
    c.shadow(&cap(), [0., 2.], 2., [0., 0., 0., 0.45]);
    c.paint(
        &cap(),
        vgrad(hex(0xf0584a), BUTTON_RED_DARK, y - ph / 2., y + ph / 2.),
    );
    c.paint(
        &round_rect(
            x - pw / 2. + 6.,
            y - ph / 2. + 3.,
            pw - 12.,
            ph * 0.38,
            ph * 0.2,
        ),
        |_, yy| {
            [
                1.,
                1.,
                1.,
                0.4 * (1.0 - (yy - (y - ph / 2. + 3.)) / (ph * 0.38)),
            ]
        },
    );
}

fn hole(c: &mut Canvas, at: [f32; 2]) {
    c.fill(&circle(at, HOLE_R + 1.5), [1., 1., 1., 0.5]);
    c.fill(&circle(at, HOLE_R), hex(0x1c1c1a));
    c.fill(&circle([at[0], at[1] + 1.], HOLE_R * 0.55), hex(0xa82a20));
    c.inner_shadow(&circle(at, HOLE_R), [0., 2.], 2., [0., 0., 0., 0.8]);
}

fn bell_icon(c: &mut Canvas, at: [f32; 2]) {
    let [x, y] = at;
    let outline = poly(&[
        [x - 5., y - 10.],
        [x + 5., y - 10.],
        [x + 8., y + 4.],
        [x + 12., y + 9.],
        [x - 12., y + 9.],
        [x - 8., y + 4.],
    ])
    .union(circle([x, y - 10.], 5.));
    let inner = poly(&[
        [x - 3.5, y - 9.],
        [x + 3.5, y - 9.],
        [x + 6., y + 4.5],
        [x + 8., y + 7.],
        [x - 8., y + 7.],
        [x - 6., y + 4.5],
    ])
    .union(circle([x, y - 10.], 3.2));
    c.fill(&outline.minus(inner), PRINT);
    c.fill(&circle([x, y + 12.], 2.4), PRINT);
}

fn clock_icon(c: &mut Canvas, at: [f32; 2]) {
    let [x, y] = at;
    // A dotted dial and two hands.
    for i in 0..9 {
        let a = -2.6 + i as f32 * 0.42;
        c.fill(&circle([x + 12. * a.cos(), y + 12. * a.sin()], 1.6), PRINT);
    }
    c.fill(&capsule([x, y], [x, y - 9.], 2.6), PRINT);
    c.fill(&capsule([x, y], [x + 8., y + 1.], 2.6), PRINT);
}

/// Shading masks drawn over buttons while they're held.
pub fn press_masks() -> Vec<(&'static str, RgbaImage)> {
    let mut out = Vec::new();
    let r = DOME_R;
    let mut dome = Canvas::at(64, 64, [-32., -32.]);
    dome.paint(&circle([0., 0.], r), |x, y| {
        // Darker toward the top, as if the cap tipped into its well.
        let t = ((y + r) / (2. * r)).clamp(0., 1.);
        let edge = ((x * x + y * y).sqrt() / r).powi(3);
        [1., 1., 1., (0.75 - 0.35 * t + 0.25 * edge).min(1.0)]
    });
    out.push(("press_dome", dome.img));
    let [pw, ph] = PILL;
    let mut pill = Canvas::at(pw as u32, ph as u32, [-pw / 2., -ph / 2.]);
    pill.fill(
        &round_rect(-pw / 2., -ph / 2., pw, ph, ph / 2.),
        [1., 1., 1., 1.],
    );
    out.push(("press_pill", pill.img));
    let mut pin = Canvas::at(18, 18, [-9., -9.]);
    pin.fill(&circle([0., 0.], HOLE_R), [1., 1., 1., 1.]);
    out.push(("press_pin", pin.img));
    out
}
