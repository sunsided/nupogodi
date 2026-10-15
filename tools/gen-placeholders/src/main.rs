//! Draws placeholder art for the LCD segments, the printed LCD background and the device
//! bezel, and writes `assets/layout.json` (where each segment sits on the LCD and where the
//! buttons are on the bezel).
//!
//! Existing files are left alone so hand-drawn art is never overwritten; pass `--force` to
//! regenerate everything.
//!
//! ```sh
//! cargo run -p gen-placeholders [-- --force]
//! ```
//!
//! Segment coordinates are approximate positions of the IM-02's LCD segments, scaled to a
//! 660×416 LCD. Each segment is drawn centered on its layout position.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use serde_json::{Map, Value, json};

const LCD_W: u32 = 660;
const LCD_H: u32 = 416;
const DEVICE_W: u32 = 1120;
const DEVICE_H: u32 = 672;
const LCD_ORIGIN: [i32; 2] = [230, 70];

const INK: Rgba<u8> = Rgba([24, 26, 20, 255]);

fn main() {
    let force = std::env::args().any(|a| a == "--force");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let segs = root.join("art/segments");
    std::fs::create_dir_all(&segs).expect("create art/segments");

    let mut layout = Map::new();
    let mut put = |name: &str, sprite: &str, at: [f32; 2]| {
        layout.insert(name.into(), json!({ "sprite": sprite, "at": at }));
    };
    let mut sprites: Vec<(String, RgbaImage)> = Vec::new();
    let mut sprite = |name: &str, img: RgbaImage| sprites.push((name.into(), img));

    // Eggs: five positions per ramp, top to bottom.
    let eggs: [(&str, [[f32; 2]; 5]); 4] = [
        (
            "tl",
            [
                [64., 132.],
                [84., 144.],
                [108., 160.],
                [129., 170.],
                [148., 186.],
            ],
        ),
        (
            "bl",
            [
                [56., 230.],
                [79., 242.],
                [105., 253.],
                [127., 266.],
                [148., 280.],
            ],
        ),
        (
            "tr",
            [
                [596., 132.],
                [576., 144.],
                [552., 160.],
                [531., 170.],
                [512., 186.],
            ],
        ),
        (
            "br",
            [
                [604., 230.],
                [581., 242.],
                [555., 253.],
                [533., 266.],
                [512., 280.],
            ],
        ),
    ];
    sprite("egg", egg(16, 13));
    for (ramp, pos) in eggs {
        for (i, at) in pos.into_iter().enumerate() {
            put(&format!("egg_{ramp}_{i}"), "egg", at);
        }
    }

    // Wolf: two bodies and four arms with the basket.
    sprite("wolf_l", wolf(false));
    sprite("wolf_r", wolf(true));
    put("wolf_l", "wolf_l", [286., 282.]);
    put("wolf_r", "wolf_r", [374., 282.]);
    sprite("arm_tl", arm(false, true));
    sprite("arm_bl", arm(false, false));
    sprite("arm_tr", arm(true, true));
    sprite("arm_br", arm(true, false));
    put("arm_tl", "arm_tl", [204., 200.]);
    put("arm_bl", "arm_bl", [204., 290.]);
    put("arm_tr", "arm_tr", [456., 200.]);
    put("arm_br", "arm_br", [456., 290.]);

    // Floor: falling egg, broken egg, hatched chick, running chick (0 nearest, 2 at the edge).
    sprite("falling", egg(16, 13));
    sprite("broken", broken());
    sprite("hatched", hatched());
    sprite("chick", chick(false));
    sprite("chick_r", chick(true));
    put("falling_l", "falling", [150., 318.]);
    put("falling_r", "falling", [510., 318.]);
    put("broken_l", "broken", [160., 384.]);
    put("broken_r", "broken", [500., 384.]);
    put("hatched_l", "hatched", [160., 378.]);
    put("hatched_r", "hatched", [500., 378.]);
    for (i, x) in [118.0f32, 78., 36.].into_iter().enumerate() {
        put(&format!("chick_l_{i}"), "chick", [x, 382.]);
        put(&format!("chick_r_{i}"), "chick_r", [660. - x, 382.]);
    }

    // Hare in the top-left window and his alarm bell.
    sprite("hare", hare());
    put("hare", "hare", [142., 62.]);
    sprite("bell_0", bell(-0.3));
    sprite("bell_1", bell(0.3));
    put("bell_0", "bell_0", [222., 42.]);
    put("bell_1", "bell_1", [222., 42.]);

    // Miss icons.
    sprite("miss", chick(false));
    for (i, x) in [382.0f32, 424., 466.].into_iter().enumerate() {
        put(&format!("miss_{i}"), "miss", [x, 122.]);
    }

    // Digits: a leading "1" (segments b and c) plus three seven-segment cells, and a colon.
    for s in ["a", "d", "g"] {
        sprite(&format!("seg_{s}"), bar(22, 6));
    }
    for s in ["b", "c", "e", "f"] {
        sprite(&format!("seg_{s}"), bar(6, 22));
    }
    let cy = 52.0;
    put("one_b", "seg_b", [384., cy - 12.]);
    put("one_c", "seg_c", [384., cy + 12.]);
    for (i, cx) in [416.0f32, 466., 500.].into_iter().enumerate() {
        for (s, dx, dy) in [
            ("a", 0., -24.),
            ("b", 12., -12.),
            ("c", 12., 12.),
            ("d", 0., 24.),
            ("e", -12., 12.),
            ("f", -12., -12.),
            ("g", 0., 0.),
        ] {
            put(
                &format!("d{i}_{s}"),
                &format!("seg_{s}"),
                [cx + dx, cy + dy],
            );
        }
    }
    sprite("colon", colon());
    put("colon", "colon", [441., cy]);

    // Labels and indicators.
    sprite("am", text("ДП", 2));
    sprite("pm", text("ПП", 2));
    put("am", "am", [344., 36.]);
    put("pm", "pm", [344., 66.]);
    sprite("game_a", text("ИГРА А", 2));
    sprite("game_b", text("ИГРА Б", 2));
    put("game_a", "game_a", [62., 404.]);
    put("game_b", "game_b", [598., 404.]);
    sprite("alarm", bell(0.0));
    put("alarm", "alarm", [344., 96.]);

    for (name, img) in &sprites {
        write_png(&segs.join(format!("{name}.png")), img, force);
    }
    write_png(&root.join("art/lcd_bg.png"), &lcd_background(), force);
    write_png(&root.join("art/bezel.png"), &bezel(), force);

    let buttons = json!({
        "ramp_tl": { "circle": [120, 380, 50] },
        "ramp_bl": { "circle": [120, 548, 50] },
        "ramp_tr": { "circle": [1000, 380, 50] },
        "ramp_br": { "circle": [1000, 548, 50] },
        "game_a": { "rect": [955, 96, 90, 30] },
        "game_b": { "rect": [955, 156, 90, 30] },
        "time": { "rect": [955, 216, 90, 30] },
        "alarm_set": { "circle": [92, 130, 13] },
        "time_set": { "circle": [148, 130, 13] }
    });
    let doc = json!({
        "device": [DEVICE_W, DEVICE_H],
        "lcd_origin": LCD_ORIGIN,
        "lcd_size": [LCD_W, LCD_H],
        "segments": Value::Object(layout),
        "buttons": buttons,
    });
    let path = root.join("assets/layout.json");
    if force || !path.exists() {
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).expect("layout");
        println!("wrote {}", path.display());
    }
}

fn write_png(path: &Path, img: &RgbaImage, force: bool) {
    if !force && path.exists() {
        return;
    }
    img.save(path)
        .unwrap_or_else(|e| panic!("save {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

// --- tiny rasterizer ---------------------------------------------------------------------

struct Canvas(RgbaImage);

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self(RgbaImage::new(w, h))
    }

    fn fill(mut self, c: Rgba<u8>) -> Self {
        for p in self.0.pixels_mut() {
            *p = c;
        }
        self
    }

    fn plot(&mut self, x: i32, y: i32, c: Rgba<u8>) {
        if x >= 0 && y >= 0 && (x as u32) < self.0.width() && (y as u32) < self.0.height() {
            let dst = self.0.get_pixel_mut(x as u32, y as u32);
            let a = c[3] as f32 / 255.0;
            for i in 0..3 {
                dst[i] = (c[i] as f32 * a + dst[i] as f32 * (1.0 - a)) as u8;
            }
            dst[3] = dst[3].max(c[3]);
        }
    }

    /// Filled ellipse centered at (cx, cy) with radii (rx, ry), rotated by `rot` radians.
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, c: Rgba<u8>) {
        let r = rx.max(ry).ceil() as i32 + 1;
        let (s, co) = rot.sin_cos();
        for y in (cy as i32 - r)..=(cy as i32 + r) {
            for x in (cx as i32 - r)..=(cx as i32 + r) {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let u = dx * co + dy * s;
                let v = -dx * s + dy * co;
                if (u / rx).powi(2) + (v / ry).powi(2) <= 1.0 {
                    self.plot(x, y, c);
                }
            }
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba<u8>) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.plot(xx, yy, c);
            }
        }
    }

    /// Thick line with round caps.
    fn line(&mut self, a: [f32; 2], b: [f32; 2], width: f32, c: Rgba<u8>) {
        let r = width / 2.0;
        let minx = a[0].min(b[0]) - r;
        let maxx = a[0].max(b[0]) + r;
        let miny = a[1].min(b[1]) - r;
        let maxy = a[1].max(b[1]) + r;
        let d = [b[0] - a[0], b[1] - a[1]];
        let len2 = (d[0] * d[0] + d[1] * d[1]).max(1e-6);
        for y in miny as i32..=maxy as i32 {
            for x in minx as i32..=maxx as i32 {
                let p = [x as f32 + 0.5 - a[0], y as f32 + 0.5 - a[1]];
                let t = ((p[0] * d[0] + p[1] * d[1]) / len2).clamp(0.0, 1.0);
                let q = [p[0] - d[0] * t, p[1] - d[1] * t];
                if q[0] * q[0] + q[1] * q[1] <= r * r {
                    self.plot(x, y, c);
                }
            }
        }
    }

    fn polygon(&mut self, pts: &[[f32; 2]], c: Rgba<u8>) {
        let miny = pts.iter().map(|p| p[1]).fold(f32::MAX, f32::min) as i32;
        let maxy = pts.iter().map(|p| p[1]).fold(f32::MIN, f32::max) as i32;
        for y in miny..=maxy {
            let fy = y as f32 + 0.5;
            let mut xs = Vec::new();
            for i in 0..pts.len() {
                let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
                if (p[1] <= fy) != (q[1] <= fy) {
                    xs.push(p[0] + (fy - p[1]) / (q[1] - p[1]) * (q[0] - p[0]));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.chunks(2) {
                if let [x0, x1] = pair {
                    for x in x0.round() as i32..x1.round() as i32 {
                        self.plot(x, y, c);
                    }
                }
            }
        }
    }

    fn mirror(self) -> Self {
        Self(image::imageops::flip_horizontal(&self.0))
    }
}

// --- segment shapes ------------------------------------------------------------------------

fn egg(w: u32, h: u32) -> RgbaImage {
    let mut c = Canvas::new(w, h);
    c.ellipse(
        w as f32 / 2.0,
        h as f32 / 2.0,
        w as f32 / 2.0,
        h as f32 / 2.0,
        0.4,
        INK,
    );
    c.0
}

fn bar(w: u32, h: u32) -> RgbaImage {
    let mut c = Canvas::new(w, h);
    let r = w.min(h) as f32 / 2.0;
    if w > h {
        c.line([r, r], [w as f32 - r, r], w.min(h) as f32, INK);
    } else {
        c.line([r, r], [r, h as f32 - r], w.min(h) as f32, INK);
    }
    c.0
}

fn colon() -> RgbaImage {
    let mut c = Canvas::new(6, 22);
    c.ellipse(3.0, 4.0, 3.0, 3.0, 0.0, INK);
    c.ellipse(3.0, 18.0, 3.0, 3.0, 0.0, INK);
    c.0
}

/// Wolf body facing left (or right when mirrored): head with snout and ears, torso, legs.
fn wolf(right: bool) -> RgbaImage {
    let mut c = Canvas::new(110, 196);
    c.ellipse(58.0, 34.0, 22.0, 24.0, 0.0, INK); // head
    c.ellipse(34.0, 44.0, 20.0, 9.0, -0.2, INK); // snout
    c.polygon(&[[48.0, 14.0], [52.0, 0.0], [60.0, 12.0]], INK); // ears
    c.polygon(&[[64.0, 12.0], [72.0, 0.0], [76.0, 16.0]], INK);
    c.ellipse(60.0, 104.0, 30.0, 46.0, 0.0, INK); // torso
    c.line([50.0, 140.0], [36.0, 190.0], 14.0, INK); // legs
    c.line([70.0, 140.0], [84.0, 190.0], 14.0, INK);
    c.line([30.0, 192.0], [46.0, 192.0], 8.0, INK); // feet
    c.line([80.0, 192.0], [96.0, 192.0], 8.0, INK);
    c.line([88.0, 120.0], [104.0, 150.0], 8.0, INK); // tail
    let c = if right { c.mirror() } else { c };
    c.0
}

/// Arm holding the basket out toward a ramp end. Drawn for the left side, mirrored for right.
fn arm(right: bool, top: bool) -> RgbaImage {
    let mut c = Canvas::new(88, 56);
    let (shoulder, hand) = if top {
        ([84.0, 50.0], [26.0, 20.0])
    } else {
        ([84.0, 8.0], [26.0, 30.0])
    };
    c.line(shoulder, hand, 9.0, INK);
    // Basket: a bowl under the hand.
    c.polygon(
        &[
            [2.0, hand[1] - 2.0],
            [36.0, hand[1] - 2.0],
            [30.0, hand[1] + 18.0],
            [8.0, hand[1] + 18.0],
        ],
        INK,
    );
    let c = if right { c.mirror() } else { c };
    c.0
}

fn hare() -> RgbaImage {
    let mut c = Canvas::new(104, 116);
    c.ellipse(26.0, 26.0, 7.0, 26.0, -0.25, INK); // ears
    c.ellipse(44.0, 24.0, 7.0, 24.0, 0.15, INK);
    c.ellipse(38.0, 64.0, 22.0, 18.0, 0.0, INK); // head
    c.ellipse(50.0, 100.0, 30.0, 16.0, 0.0, INK); // shoulders
    c.line([66.0, 88.0], [96.0, 50.0], 8.0, INK); // raised arm
    c.0
}

fn bell(tilt: f32) -> RgbaImage {
    let mut c = Canvas::new(28, 28);
    c.ellipse(14.0, 13.0, 9.0, 10.0, tilt, INK);
    c.line(
        [4.0 + tilt * 6.0, 22.0],
        [24.0 + tilt * 6.0, 22.0],
        4.0,
        INK,
    );
    c.ellipse(14.0 + tilt * 8.0, 25.0, 3.0, 3.0, 0.0, INK);
    c.0
}

fn broken() -> RgbaImage {
    let mut c = Canvas::new(56, 22);
    c.polygon(
        &[
            [2.0, 20.0],
            [10.0, 10.0],
            [16.0, 14.0],
            [22.0, 2.0],
            [28.0, 12.0],
            [36.0, 4.0],
            [40.0, 14.0],
            [54.0, 20.0],
        ],
        INK,
    );
    c.0
}

fn hatched() -> RgbaImage {
    let mut c = Canvas::new(40, 34);
    c.ellipse(20.0, 12.0, 9.0, 9.0, 0.0, INK); // chick head
    c.polygon(
        &[
            [4.0, 18.0],
            [10.0, 24.0],
            [16.0, 18.0],
            [22.0, 24.0],
            [28.0, 18.0],
            [36.0, 18.0],
            [32.0, 32.0],
            [8.0, 32.0],
        ],
        INK,
    ); // shell
    c.0
}

fn chick(right: bool) -> RgbaImage {
    let mut c = Canvas::new(30, 28);
    c.ellipse(16.0, 16.0, 11.0, 8.0, 0.0, INK);
    c.ellipse(8.0, 8.0, 6.0, 6.0, 0.0, INK);
    c.polygon(&[[2.0, 7.0], [-2.0, 9.0], [2.0, 11.0]], INK);
    c.line([14.0, 22.0], [10.0, 27.0], 3.0, INK);
    c.line([20.0, 22.0], [24.0, 27.0], 3.0, INK);
    let c = if right { c.mirror() } else { c };
    c.0
}

/// A 5×7 bitmap font with just the Cyrillic letters the LCD labels need.
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'Д' => [0x0E, 0x0A, 0x0A, 0x0A, 0x0A, 0x1F, 0x11],
        'П' => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11],
        'И' => [0x11, 0x11, 0x13, 0x15, 0x19, 0x11, 0x11],
        'Г' => [0x1F, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10],
        'Р' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'А' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'Б' => [0x1F, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x1E],
        _ => [0; 7],
    }
}

fn text(s: &str, scale: u32) -> RgbaImage {
    let n = s.chars().count() as u32;
    let mut c = Canvas::new((n * 6 - 1) * scale, 7 * scale);
    for (i, ch) in s.chars().enumerate() {
        for (row, bits) in glyph(ch).into_iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    c.rect(
                        ((i as u32 * 6 + col) * scale) as i32,
                        (row as u32 * scale) as i32,
                        scale as i32,
                        scale as i32,
                        INK,
                    );
                }
            }
        }
    }
    c.0
}

// --- printed background and bezel -----------------------------------------------------

fn lcd_background() -> RgbaImage {
    let mut c = Canvas::new(LCD_W, LCD_H);
    // Reflective LCD: a pale warm grey with a soft vertical gradient.
    for y in 0..LCD_H {
        let t = y as f32 / LCD_H as f32;
        let v = |a: f32, b: f32| (a + (b - a) * t) as u8;
        for x in 0..LCD_W {
            c.0.put_pixel(
                x,
                y,
                Rgba([v(206.0, 186.0), v(206.0, 188.0), v(182.0, 166.0), 255]),
            );
        }
    }
    let wood = Rgba([150, 98, 56, 255]);
    let hen = Rgba([248, 246, 236, 255]);
    let comb = Rgba([200, 40, 30, 255]);
    let grass = Rgba([96, 150, 70, 255]);
    // Ramps under the egg paths.
    for (a, b) in [
        ([40.0, 146.0], [166.0, 204.0]),
        ([32.0, 244.0], [166.0, 298.0]),
        ([620.0, 146.0], [494.0, 204.0]),
        ([628.0, 244.0], [494.0, 298.0]),
    ] {
        c.line(a, b, 6.0, wood);
    }
    // Hens on their roosts at the top of each ramp.
    for (x, y, flip) in [
        (28.0, 118.0, false),
        (22.0, 216.0, false),
        (632.0, 118.0, true),
        (638.0, 216.0, true),
    ] {
        let d = if flip { -1.0 } else { 1.0 };
        c.ellipse(x, y, 18.0, 13.0, 0.0, hen);
        c.ellipse(x + 12.0 * d, y - 12.0, 7.0, 7.0, 0.0, hen);
        c.ellipse(x + 12.0 * d, y - 20.0, 4.0, 3.0, 0.0, comb);
        c.rect((x - 22.0) as i32, (y + 12.0) as i32, 44, 5, wood);
    }
    // Grass along the bottom.
    c.rect(0, LCD_H as i32 - 26, LCD_W as i32, 26, grass);
    c.0
}

fn bezel() -> RgbaImage {
    let body = Rgba([228, 222, 208, 255]);
    let edge = Rgba([196, 188, 172, 255]);
    let frame = Rgba([40, 38, 36, 255]);
    let red = Rgba([184, 50, 42, 255]);
    let red_dark = Rgba([130, 30, 26, 255]);
    let hole = Rgba([70, 66, 60, 255]);

    let mut c = Canvas::new(DEVICE_W, DEVICE_H).fill(Rgba([0, 0, 0, 0]));
    // Rounded body.
    let r = 40.0;
    let (w, h) = (DEVICE_W as f32, DEVICE_H as f32);
    c.rect(r as i32, 0, (w - 2.0 * r) as i32, h as i32, edge);
    c.rect(0, r as i32, w as i32, (h - 2.0 * r) as i32, edge);
    for (x, y) in [(r, r), (w - r, r), (r, h - r), (w - r, h - r)] {
        c.ellipse(x, y, r, r, 0.0, edge);
    }
    let i = 8.0;
    c.rect(
        (r) as i32,
        i as i32,
        (w - 2.0 * r) as i32,
        (h - 2.0 * i) as i32,
        body,
    );
    c.rect(
        i as i32,
        r as i32,
        (w - 2.0 * i) as i32,
        (h - 2.0 * r) as i32,
        body,
    );
    for (x, y) in [(r, r), (w - r, r), (r, h - r), (w - r, h - r)] {
        c.ellipse(x, y, r - i, r - i, 0.0, body);
    }
    // Dark frame around the LCD.
    let [ox, oy] = LCD_ORIGIN;
    c.rect(
        ox - 18,
        oy - 18,
        LCD_W as i32 + 36,
        LCD_H as i32 + 36,
        frame,
    );
    // Big buttons.
    for (x, y) in [
        (120.0, 380.0),
        (120.0, 548.0),
        (1000.0, 380.0),
        (1000.0, 548.0),
    ] {
        c.ellipse(x, y + 4.0, 52.0, 52.0, 0.0, red_dark);
        c.ellipse(x, y, 50.0, 50.0, 0.0, red);
    }
    // Small buttons.
    for y in [96, 156, 216] {
        c.rect(955, y + 3, 90, 30, red_dark);
        c.rect(955, y, 90, 30, red);
    }
    // Recessed set buttons.
    for x in [92.0, 148.0] {
        c.ellipse(x, 130.0, 13.0, 13.0, 0.0, hole);
    }
    c.0
}
