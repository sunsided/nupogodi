//! Draw the bezel, the printed LCD background, the segments and the glass over them.
//! The printed labels are part of the bezel art.

use game_toolkit::prelude::*;

use crate::atlas::Atlas;
use crate::controls::{Button, View2D};
use crate::layout::{Layout, Shape};

/// Unlit segments are faintly visible on a real LCD.
const GHOST: [f32; 4] = [1.0, 1.0, 1.0, 0.045];
const LIT: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
/// Lit segments cast a soft shadow onto the mirror behind the liquid crystal.
const SHADOW: [f32; 4] = [1.0, 1.0, 1.0, 0.14];
const SHADOW_OFFSET: [f32; 2] = [2.5, 3.0];

/// Held buttons are shaded with a mask sprite from the atlas.
const PRESSED: [f32; 4] = [0.0, 0.0, 0.0, 0.32];

mod layer {
    pub const BEZEL: i16 = 0;
    pub const LCD: i16 = 1;
    pub const GHOST: i16 = 2;
    pub const SHADOW: i16 = 3;
    pub const LIT: i16 = 4;
    pub const GLASS: i16 = 5;
    pub const PRESSED: i16 = 6;
}

pub struct Renderer {
    pub atlas: Atlas,
    bezel: TextureId,
    lcd_bg: TextureId,
    lcd_glass: TextureId,
}

impl Renderer {
    pub fn load(ctx: &mut Context) -> Result<Self> {
        Ok(Self {
            atlas: Atlas::load(ctx, "atlas.png", "atlas.json")?,
            bezel: ctx.load_texture("bezel.png")?,
            lcd_bg: ctx.load_texture("lcd_bg.png")?,
            lcd_glass: ctx.load_texture("lcd_glass.png")?,
        })
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        frame: &mut Frame,
        layout: &Layout,
        view: &View2D,
        lit: &[String],
        held: &[Button],
    ) {
        ctx.gfx.camera.zoom = view.zoom;
        ctx.gfx.camera.center = [layout.device[0] * 0.5, layout.device[1] * 0.5];

        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.0, 0.0, 0.0, 0.0]);

        let quad = |p: &mut Painter, tex, pos, size, layer| {
            p.sprite_ex(tex, SpriteInstance::at(pos, size), layer, BlendMode::Alpha);
        };
        quad(&mut p, self.bezel, [0.0, 0.0], layout.device, layer::BEZEL);
        quad(
            &mut p,
            self.lcd_bg,
            layout.lcd_origin,
            layout.lcd_size,
            layer::LCD,
        );

        let segment = |p: &mut Painter, name: &str, color, layer, offset: [f32; 2]| {
            let Some(place) = layout.segments.get(name) else {
                return;
            };
            let Some(s) = self.atlas.get(&place.sprite) else {
                return;
            };
            let pos = [
                layout.lcd_origin[0] + place.at[0] - s.size[0] * 0.5 + offset[0],
                layout.lcd_origin[1] + place.at[1] - s.size[1] * 0.5 + offset[1],
            ];
            let inst = SpriteInstance::at(pos, s.size)
                .with_uv(s.uv_min, s.uv_max)
                .with_color(color);
            p.sprite_ex(self.atlas.texture, inst, layer, BlendMode::Alpha);
        };
        for name in layout.segments.keys() {
            segment(&mut p, name, GHOST, layer::GHOST, [0.0, 0.0]);
        }
        for name in lit {
            segment(&mut p, name, SHADOW, layer::SHADOW, SHADOW_OFFSET);
            segment(&mut p, name, LIT, layer::LIT, [0.0, 0.0]);
        }

        quad(
            &mut p,
            self.lcd_glass,
            layout.lcd_origin,
            layout.lcd_size,
            layer::GLASS,
        );

        // Held buttons sink into their sockets.
        for b in held {
            let (center, mask) = match layout.buttons.get(b.name()) {
                Some(Shape::Circle([x, y, r])) if *r > 20.0 => ([*x, *y], "press_dome"),
                Some(Shape::Circle([x, y, _])) => ([*x, *y], "press_pin"),
                Some(Shape::Rect([x, y, w, h])) => ([x + w * 0.5, y + h * 0.5], "press_pill"),
                None => continue,
            };
            let Some(s) = self.atlas.get(mask) else {
                continue;
            };
            let pos = [center[0] - s.size[0] * 0.5, center[1] - s.size[1] * 0.5];
            let inst = SpriteInstance::at(pos, s.size)
                .with_uv(s.uv_min, s.uv_max)
                .with_color(PRESSED);
            p.sprite_ex(self.atlas.texture, inst, layer::PRESSED, BlendMode::Alpha);
        }
    }
}
