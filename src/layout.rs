//! `assets/layout.json`: where each LCD segment and each bezel button sits.

use std::collections::HashMap;

use game_toolkit::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Layout {
    /// Device (bezel) size in logical pixels; the whole game is drawn in this space.
    pub device: [f32; 2],
    /// Top-left of the LCD on the bezel.
    pub lcd_origin: [f32; 2],
    pub lcd_size: [f32; 2],
    /// Segment name → sprite and center position in LCD space.
    pub segments: HashMap<String, Placement>,
    pub buttons: HashMap<String, Shape>,
}

#[derive(Deserialize)]
pub struct Placement {
    pub sprite: String,
    pub at: [f32; 2],
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// `[cx, cy, r]`
    Circle([f32; 3]),
    /// `[x, y, w, h]`
    Rect([f32; 4]),
}

impl Shape {
    pub fn contains(&self, p: [f32; 2]) -> bool {
        match *self {
            Shape::Circle([cx, cy, r]) => {
                // A little slack makes the big buttons easy to hit with a thumb.
                let r = r * 1.15;
                (p[0] - cx).powi(2) + (p[1] - cy).powi(2) <= r * r
            }
            Shape::Rect([x, y, w, h]) => {
                p[0] >= x - 6.0 && p[0] <= x + w + 6.0 && p[1] >= y - 6.0 && p[1] <= y + h + 6.0
            }
        }
    }
}

impl Layout {
    pub fn load(ctx: &Context, path: &str) -> Result<Self> {
        Ok(serde_json::from_slice(&ctx.assets.read(path)?)?)
    }
}
