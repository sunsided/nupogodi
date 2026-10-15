//! Draw the bezel, the printed LCD background and the segments.

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

const LABEL: [f32; 4] = [0.45, 0.10, 0.08, 1.0];
const TITLE: [f32; 4] = [0.72, 0.12, 0.10, 1.0];

mod layer {
    pub const BEZEL: i16 = 0;
    pub const LCD: i16 = 1;
    pub const GHOST: i16 = 2;
    pub const SHADOW: i16 = 3;
    pub const LIT: i16 = 4;
    pub const PRESSED: i16 = 5;
}

pub struct Renderer {
    pub atlas: Atlas,
    bezel: TextureId,
    lcd_bg: TextureId,
}

impl Renderer {
    pub fn load(ctx: &mut Context) -> Result<Self> {
        Ok(Self {
            atlas: Atlas::load(ctx, "atlas.png", "atlas.json")?,
            bezel: ctx.load_texture("bezel.png")?,
            lcd_bg: ctx.load_texture("lcd_bg.png")?,
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

        let white = ctx.gfx.white_texture();
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

        // Pressed buttons sink a little.
        for b in held {
            match layout.buttons.get(b.name()) {
                Some(Shape::Circle([x, y, r])) => {
                    p.circle_ex([*x, *y], *r, 0.0, [0.0, 0.0, 0.0, 0.28], layer::PRESSED);
                }
                Some(Shape::Rect([x, y, w, h])) => {
                    let inst =
                        SpriteInstance::at([*x, *y], [*w, *h]).with_color([0.0, 0.0, 0.0, 0.28]);
                    p.sprite_ex(white, inst, layer::PRESSED, BlendMode::Alpha);
                }
                None => {}
            }
        }

        // Printed labels. Text is drawn in window pixels, so map from device space.
        let z = view.zoom;
        let mut text = |s: &str, at: [f32; 2], size: f32, color| {
            p.text(view.to_screen(at), s, size * z, color);
        };
        let [ox, oy] = layout.lcd_origin;
        let [lw, lh] = layout.lcd_size;
        text("ЭЛЕКТРОНИКА", [ox, oy - 52.0], 22.0, LABEL);
        text(
            "НУ, ПОГОДИ!",
            [ox + lw * 0.5 - 120.0, oy + lh + 34.0],
            40.0,
            TITLE,
        );
        text("ИМ-02", [ox + lw - 70.0, oy + lh + 46.0], 20.0, LABEL);
        for (b, label) in [
            (Button::GameA, "ИГРА А"),
            (Button::GameB, "ИГРА Б"),
            (Button::Time, "ВРЕМЯ"),
        ] {
            if let Some(Shape::Rect([x, y, _, _])) = layout.buttons.get(b.name()) {
                text(label, [*x + 4.0, *y - 22.0], 15.0, LABEL);
            }
        }
        for (b, label) in [(Button::AlarmSet, "БУД."), (Button::TimeSet, "ВР.")] {
            if let Some(Shape::Circle([x, y, r])) = layout.buttons.get(b.name()) {
                text(label, [*x - 16.0, *y + r + 6.0], 13.0, LABEL);
            }
        }
    }
}
