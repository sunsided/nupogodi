//! Printed labels, rasterized with `ab_glyph` from the vendored DejaVu fonts.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};

use crate::canvas::{Canvas, Color};

pub static BOLD: &[u8] = include_bytes!("../fonts/DejaVuSans-Bold.ttf");
pub static CONDENSED: &[u8] = include_bytes!("../fonts/DejaVuSansCondensed-Bold.ttf");

pub struct Text<'a> {
    font: FontRef<'a>,
    pub size: f32,
    /// Extra space between letters, in pixels.
    pub tracking: f32,
}

impl<'a> Text<'a> {
    pub fn new(data: &'a [u8], size: f32) -> Self {
        Self {
            font: FontRef::try_from_slice(data).expect("font"),
            size,
            tracking: 0.0,
        }
    }

    pub fn tracking(mut self, t: f32) -> Self {
        self.tracking = t;
        self
    }

    pub fn width(&self, s: &str) -> f32 {
        let f = self.font.as_scaled(PxScale::from(self.size));
        let n = s.chars().count();
        let mut w = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            let id = f.glyph_id(ch);
            if let Some(p) = prev {
                w += f.kern(p, id);
            }
            w += f.h_advance(id);
            prev = Some(id);
        }
        w + self.tracking * n.saturating_sub(1) as f32
    }

    /// Height of a capital letter, for centering text on a line.
    pub fn cap_height(&self) -> f32 {
        let f = self.font.as_scaled(PxScale::from(self.size));
        let g = f
            .outline_glyph(f.scaled_glyph('H'))
            .expect("H has an outline");
        g.px_bounds().height()
    }

    /// Draw `s` centered on `at[0]`, with its baseline at `at[1]`.
    pub fn draw(&self, c: &mut Canvas, s: &str, at: [f32; 2], color: Color) {
        let f = self.font.as_scaled(PxScale::from(self.size));
        let mut x = at[0] - self.width(s) / 2.0;
        let mut prev = None;
        for ch in s.chars() {
            let id = f.glyph_id(ch);
            if let Some(p) = prev {
                x += f.kern(p, id);
            }
            let g = id.with_scale_and_position(self.size, point(x, at[1]));
            if let Some(og) = self.font.outline_glyph(g) {
                let b = og.px_bounds();
                let (w, h) = c.img.dimensions();
                og.draw(|gx, gy, cov| {
                    let px = (b.min.x + gx as f32 - c.origin[0]).floor();
                    let py = (b.min.y + gy as f32 - c.origin[1]).floor();
                    if px >= 0.0 && py >= 0.0 && (px as u32) < w && (py as u32) < h {
                        c.blend(px as u32, py as u32, color, cov.min(1.0));
                    }
                });
            }
            x += f.h_advance(id) + self.tracking;
            prev = Some(id);
        }
    }
}
