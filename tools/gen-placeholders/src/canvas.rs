//! A tiny anti-aliased rasterizer: shapes are point-in-shape tests, filled with 4×4
//! supersampling. Good enough for flat print-and-LCD art, and needs no extra crates.

use image::{Rgba, RgbaImage};

/// Straight-alpha color, all channels in `0.0..=1.0`.
pub type Color = [f32; 4];

pub const fn hex(c: u32) -> Color {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
        1.0,
    ]
}

pub const fn alpha(c: Color, a: f32) -> Color {
    [c[0], c[1], c[2], a]
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// Cheap deterministic value noise in `0.0..1.0`.
pub fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65536.0
}

type Test = Box<dyn Fn(f32, f32) -> bool>;

/// A region of the plane: a bounding box plus an inside test.
pub struct Shape {
    bbox: [f32; 4],
    inside: Test,
}

impl Shape {
    pub fn new(bbox: [f32; 4], inside: impl Fn(f32, f32) -> bool + 'static) -> Self {
        Self {
            bbox,
            inside: Box::new(inside),
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.bbox[0]
            && y >= self.bbox[1]
            && x <= self.bbox[2]
            && y <= self.bbox[3]
            && (self.inside)(x, y)
    }

    pub fn union(self, o: Shape) -> Shape {
        let b = [
            self.bbox[0].min(o.bbox[0]),
            self.bbox[1].min(o.bbox[1]),
            self.bbox[2].max(o.bbox[2]),
            self.bbox[3].max(o.bbox[3]),
        ];
        Shape::new(b, move |x, y| self.contains(x, y) || o.contains(x, y))
    }

    pub fn minus(self, o: Shape) -> Shape {
        let b = self.bbox;
        Shape::new(b, move |x, y| self.contains(x, y) && !o.contains(x, y))
    }

    pub fn and(self, o: Shape) -> Shape {
        let b = [
            self.bbox[0].max(o.bbox[0]),
            self.bbox[1].max(o.bbox[1]),
            self.bbox[2].min(o.bbox[2]),
            self.bbox[3].min(o.bbox[3]),
        ];
        Shape::new(b, move |x, y| self.contains(x, y) && o.contains(x, y))
    }
}

pub fn union_all(shapes: impl IntoIterator<Item = Shape>) -> Shape {
    shapes
        .into_iter()
        .reduce(Shape::union)
        .unwrap_or_else(|| Shape::new([0.0; 4], |_, _| false))
}

pub fn circle(c: [f32; 2], r: f32) -> Shape {
    ellipse(c, r, r, 0.0)
}

/// Ellipse with radii `(rx, ry)` rotated by `rot` radians.
pub fn ellipse(c: [f32; 2], rx: f32, ry: f32, rot: f32) -> Shape {
    let m = rx.max(ry);
    let (s, co) = rot.sin_cos();
    Shape::new([c[0] - m, c[1] - m, c[0] + m, c[1] + m], move |x, y| {
        let (dx, dy) = (x - c[0], y - c[1]);
        let u = dx * co + dy * s;
        let v = -dx * s + dy * co;
        (u / rx).powi(2) + (v / ry).powi(2) <= 1.0
    })
}

/// Elliptical ring of the given stroke width.
pub fn ring(c: [f32; 2], rx: f32, ry: f32, rot: f32, width: f32) -> Shape {
    ellipse(c, rx, ry, rot).minus(ellipse(c, rx - width, ry - width, rot))
}

pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Shape {
    Shape::new([x, y, x + w, y + h], |_, _| true)
}

pub fn round_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Shape {
    let r = r.min(w / 2.0).min(h / 2.0);
    Shape::new([x, y, x + w, y + h], move |px, py| {
        let cx = px.clamp(x + r, x + w - r);
        let cy = py.clamp(y + r, y + h - r);
        (px - cx).powi(2) + (py - cy).powi(2) <= r * r
    })
}

/// Outline of a rounded rectangle, `width` thick, drawn inside the rectangle.
pub fn round_frame(x: f32, y: f32, w: f32, h: f32, r: f32, width: f32) -> Shape {
    round_rect(x, y, w, h, r).minus(round_rect(
        x + width,
        y + width,
        w - 2.0 * width,
        h - 2.0 * width,
        (r - width).max(0.0),
    ))
}

/// Thick segment with round caps.
pub fn capsule(a: [f32; 2], b: [f32; 2], width: f32) -> Shape {
    tapered(a, b, width, width)
}

/// Segment whose width goes from `wa` at `a` to `wb` at `b`, with round ends.
pub fn tapered(a: [f32; 2], b: [f32; 2], wa: f32, wb: f32) -> Shape {
    let m = wa.max(wb) / 2.0;
    let bbox = [
        a[0].min(b[0]) - m,
        a[1].min(b[1]) - m,
        a[0].max(b[0]) + m,
        a[1].max(b[1]) + m,
    ];
    let d = [b[0] - a[0], b[1] - a[1]];
    let len2 = (d[0] * d[0] + d[1] * d[1]).max(1e-6);
    Shape::new(bbox, move |x, y| {
        let p = [x - a[0], y - a[1]];
        let t = ((p[0] * d[0] + p[1] * d[1]) / len2).clamp(0.0, 1.0);
        let q = [p[0] - d[0] * t, p[1] - d[1] * t];
        let r = (wa + (wb - wa) * t) / 2.0;
        q[0] * q[0] + q[1] * q[1] <= r * r
    })
}

/// A chain of capsules through `pts`, width interpolated from `w0` to `w1`.
pub fn stroke(pts: &[[f32; 2]], w0: f32, w1: f32) -> Shape {
    let n = pts.len().saturating_sub(1).max(1) as f32;
    union_all(pts.windows(2).enumerate().map(|(i, p)| {
        let wa = w0 + (w1 - w0) * i as f32 / n;
        let wb = w0 + (w1 - w0) * (i + 1) as f32 / n;
        tapered(p[0], p[1], wa, wb)
    }))
}

/// Sample a quadratic Bézier into points.
pub fn bezier(p0: [f32; 2], c: [f32; 2], p1: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            [
                u * u * p0[0] + 2.0 * u * t * c[0] + t * t * p1[0],
                u * u * p0[1] + 2.0 * u * t * c[1] + t * t * p1[1],
            ]
        })
        .collect()
}

/// Filled polygon (even-odd rule).
pub fn poly(pts: &[[f32; 2]]) -> Shape {
    let pts = pts.to_vec();
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for p in &pts {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    Shape::new(b, move |x, y| {
        let mut inside = false;
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let (pi, pj) = (pts[i], pts[j]);
            if (pi[1] > y) != (pj[1] > y)
                && x < (pj[0] - pi[0]) * (y - pi[1]) / (pj[1] - pi[1]) + pi[0]
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    })
}

/// A star with `n` points around `c`.
pub fn star(c: [f32; 2], outer: f32, inner: f32, n: usize, rot: f32) -> Shape {
    let pts: Vec<[f32; 2]> = (0..n * 2)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = rot + i as f32 * std::f32::consts::PI / n as f32;
            [c[0] + r * a.sin(), c[1] - r * a.cos()]
        })
        .collect();
    poly(&pts)
}

const SS: usize = 4;

/// An RGBA image you draw on in your own coordinate system: pixel `(0, 0)` sits at `origin`.
pub struct Canvas {
    pub img: RgbaImage,
    pub origin: [f32; 2],
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Self {
        Self::at(w, h, [0.0, 0.0])
    }

    pub fn at(w: u32, h: u32, origin: [f32; 2]) -> Self {
        Self {
            img: RgbaImage::new(w, h),
            origin,
        }
    }

    /// Fraction of the pixel `(px, py)` covered by `s`.
    fn coverage(&self, s: &Shape, px: u32, py: u32) -> f32 {
        let x0 = px as f32 + self.origin[0];
        let y0 = py as f32 + self.origin[1];
        let mut n = 0;
        for j in 0..SS {
            for i in 0..SS {
                let x = x0 + (i as f32 + 0.5) / SS as f32;
                let y = y0 + (j as f32 + 0.5) / SS as f32;
                if s.contains(x, y) {
                    n += 1;
                }
            }
        }
        n as f32 / (SS * SS) as f32
    }

    fn pixels_of(&self, s: &Shape) -> impl Iterator<Item = (u32, u32)> + use<> {
        let (w, h) = self.img.dimensions();
        let x0 = ((s.bbox[0] - self.origin[0]).floor().max(0.0) as u32).min(w);
        let y0 = ((s.bbox[1] - self.origin[1]).floor().max(0.0) as u32).min(h);
        let x1 = ((s.bbox[2] - self.origin[0]).ceil().max(0.0) as u32 + 1).min(w);
        let y1 = ((s.bbox[3] - self.origin[1]).ceil().max(0.0) as u32 + 1).min(h);
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y)))
    }

    pub fn fill(&mut self, s: &Shape, c: Color) {
        self.paint(s, |_, _| c);
    }

    /// Fill `s` with a color computed per pixel (in drawing coordinates).
    pub fn paint(&mut self, s: &Shape, f: impl Fn(f32, f32) -> Color) {
        for (x, y) in self.pixels_of(s) {
            let cov = self.coverage(s, x, y);
            if cov > 0.0 {
                let c = f(
                    x as f32 + self.origin[0] + 0.5,
                    y as f32 + self.origin[1] + 0.5,
                );
                self.blend(x, y, c, cov);
            }
        }
    }

    /// Cut `s` out of what's drawn.
    pub fn erase(&mut self, s: &Shape) {
        for (x, y) in self.pixels_of(s) {
            let cov = self.coverage(s, x, y);
            if cov > 0.0 {
                let p = self.img.get_pixel_mut(x, y);
                p[3] = (p[3] as f32 * (1.0 - cov)).round() as u8;
            }
        }
    }

    /// Source-over blend of `c` at pixel `(x, y)` with extra coverage `cov`.
    pub fn blend(&mut self, x: u32, y: u32, c: Color, cov: f32) {
        let p = self.img.get_pixel_mut(x, y);
        let sa = c[3] * cov;
        let da = p[3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        if oa <= 0.0 {
            return;
        }
        for i in 0..3 {
            let d = p[i] as f32 / 255.0;
            let o = (c[i] * sa + d * da * (1.0 - sa)) / oa;
            p[i] = (o * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        p[3] = (oa * 255.0).round() as u8;
    }

    /// Draw another canvas over this one, both in the same drawing coordinates.
    pub fn draw(&mut self, o: &Canvas, opacity: f32) {
        let dx = (o.origin[0] - self.origin[0]).round() as i64;
        let dy = (o.origin[1] - self.origin[1]).round() as i64;
        let (w, h) = self.img.dimensions();
        for (x, y, p) in o.img.enumerate_pixels() {
            let (tx, ty) = (x as i64 + dx, y as i64 + dy);
            if p[3] == 0 || tx < 0 || ty < 0 || tx >= w as i64 || ty >= h as i64 {
                continue;
            }
            let c = [
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
                p[3] as f32 / 255.0,
            ];
            self.blend(tx as u32, ty as u32, c, opacity);
        }
    }

    /// A soft shadow of `s`: its silhouette, blurred by `radius` and offset.
    pub fn shadow(&mut self, s: &Shape, offset: [f32; 2], radius: f32, c: Color) {
        let mut m = Canvas::at(
            self.img.width(),
            self.img.height(),
            [self.origin[0] - offset[0], self.origin[1] - offset[1]],
        );
        m.fill(s, c);
        m.blur(radius);
        m.origin = self.origin;
        self.draw(&m, 1.0);
    }

    /// Glow inside `s` along its edge: `c` fades in toward the boundary over `radius`, and is
    /// shifted by `offset` so the light seems to come from one side.
    pub fn inner_shadow(&mut self, s: &Shape, offset: [f32; 2], radius: f32, c: Color) {
        let (w, h) = self.img.dimensions();
        let mut m = Canvas::at(w, h, self.origin);
        m.fill(&rect(-1e4, -1e4, 2e4, 2e4), c);
        let mut hole = Canvas::at(
            w,
            h,
            [self.origin[0] - offset[0], self.origin[1] - offset[1]],
        );
        hole.fill(s, [1.0; 4]);
        for (x, y, p) in hole.img.enumerate_pixels() {
            let q = m.img.get_pixel_mut(x, y);
            q[3] = (q[3] as f32 * (1.0 - p[3] as f32 / 255.0)) as u8;
        }
        m.blur(radius);
        // Keep only what falls inside the shape.
        let mut mask = Canvas::at(w, h, self.origin);
        mask.fill(s, [1.0; 4]);
        for (x, y, p) in mask.img.enumerate_pixels() {
            let q = m.img.get_pixel_mut(x, y);
            q[3] = (q[3] as f32 * p[3] as f32 / 255.0) as u8;
        }
        self.draw(&m, 1.0);
    }

    /// Three box-blur passes (close to a Gaussian), premultiplied.
    pub fn blur(&mut self, radius: f32) {
        let r = (radius / 1.7).round().max(1.0) as i32;
        let (w, h) = (self.img.width() as i32, self.img.height() as i32);
        let mut buf: Vec<[f32; 4]> = self
            .img
            .pixels()
            .map(|p| {
                let a = p[3] as f32 / 255.0;
                [
                    p[0] as f32 / 255.0 * a,
                    p[1] as f32 / 255.0 * a,
                    p[2] as f32 / 255.0 * a,
                    a,
                ]
            })
            .collect();
        let mut tmp = buf.clone();
        let pass = |src: &[[f32; 4]], dst: &mut [[f32; 4]], horiz: bool| {
            let (n, m) = if horiz { (h, w) } else { (w, h) };
            for line in 0..n {
                let idx = |k: i32| -> usize {
                    let k = k.clamp(0, m - 1);
                    if horiz {
                        (line * w + k) as usize
                    } else {
                        (k * w + line) as usize
                    }
                };
                let mut acc = [0.0f32; 4];
                for k in -r..=r {
                    let s = src[idx(k)];
                    for i in 0..4 {
                        acc[i] += s[i];
                    }
                }
                let norm = 1.0 / (2 * r + 1) as f32;
                for k in 0..m {
                    dst[idx(k)] = acc.map(|v| v * norm);
                    let (add, sub) = (src[idx(k + r + 1)], src[idx(k - r)]);
                    for i in 0..4 {
                        acc[i] += add[i] - sub[i];
                    }
                }
            }
        };
        for _ in 0..3 {
            pass(&buf, &mut tmp, true);
            pass(&tmp, &mut buf, false);
        }
        for (p, v) in self.img.pixels_mut().zip(buf) {
            let a = v[3].clamp(0.0, 1.0);
            let un = |c: f32| {
                if a > 0.0 {
                    (c / a * 255.0).round().clamp(0.0, 255.0) as u8
                } else {
                    0
                }
            };
            *p = Rgba([un(v[0]), un(v[1]), un(v[2]), (a * 255.0).round() as u8]);
        }
    }

    /// Tight crop around everything drawn, plus the crop's center in drawing coordinates.
    pub fn crop(&self) -> (RgbaImage, [f32; 2]) {
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for (x, y, p) in self.img.enumerate_pixels() {
            if p[3] > 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
        assert!(x0 <= x1, "empty sprite");
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let img = image::imageops::crop_imm(&self.img, x0, y0, w, h).to_image();
        let center = [
            self.origin[0] + x0 as f32 + w as f32 / 2.0,
            self.origin[1] + y0 as f32 + h as f32 / 2.0,
        ];
        (img, center)
    }
}
