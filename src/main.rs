//! Ну, погоди! — a clone of the Soviet Elektronika IM-02 LCD handheld (itself a copy of
//! Nintendo's Game & Watch "Egg"). Catch the eggs rolling down four ramps; Game A, Game B,
//! a 12-hour clock with alarm, and a persisted high score.

mod atlas;
mod controls;
mod display;
mod layout;
mod logic;
mod platform;
mod render;
mod sound;

use std::time::Duration;

use game_toolkit::prelude::*;
use winit::event::WindowEvent;

use controls::{Controls, View2D};
use layout::Layout;
use logic::{Inputs, Machine, Random};
use render::Renderer;
use sound::Sounds;

/// Logical device size (the IM-02 is 112 × 67 mm).
const WIDTH: u32 = 1120;
const HEIGHT: u32 = 672;
/// A frame stall longer than this (background tab, detached canvas) pauses the game.
const STALL: f32 = 0.25;

struct Dice<'a>(&'a mut Rng);

impl Random for Dice<'_> {
    fn next_f32(&mut self) -> f32 {
        self.0.next_f32()
    }
}

struct NuPogodi {
    layout: Layout,
    renderer: Renderer,
    controls: Controls,
    sounds: Sounds,
    machine: Machine,
    view: View2D,
    last_input: Inputs,
}

impl Game for NuPogodi {
    fn init(ctx: &mut Context) -> Result<Self> {
        let layout = Layout::load(ctx, "layout.json")?;
        let renderer = Renderer::load(ctx)?;
        let view = View2D::fit(ctx.gfx.size(), layout.device);
        Ok(Self {
            layout,
            renderer,
            controls: Controls::default(),
            sounds: Sounds::new(),
            machine: Machine::new(platform::seconds_of_day(), platform::load_hiscore()),
            view,
            last_input: Inputs::default(),
        })
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        if ctx.input.key_pressed(Key::Escape) {
            ctx.quit();
        }
        if ctx.input.key_pressed(Key::KeyM) {
            self.sounds.muted = !self.sounds.muted;
        }

        if ctx.time.delta.as_secs_f32() > STALL || !platform::is_active() {
            self.machine.tick_clock(dt);
            return;
        }

        let input = self.controls.poll(&ctx.input, &self.view, &self.layout);

        #[cfg(debug_assertions)]
        if ctx.input.key_pressed(Key::KeyN) && self.machine.is_playing() {
            self.machine.round.total += 10;
        }

        let cues = self.machine.update(dt, &input, &mut Dice(&mut ctx.rng));
        for cue in cues {
            self.sounds.play(ctx.audio.as_mut(), cue);
        }
        if std::mem::take(&mut self.machine.hiscore_dirty) {
            platform::save_hiscore(self.machine.hiscore);
        }
        self.last_input = input;
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let lit = display::lit_segments(&self.machine.view(&self.last_input));
        let held = self.controls.visually_held(&ctx.input);
        self.renderer
            .draw(ctx, frame, &self.layout, &self.view, &lit, &held);
    }

    fn event(&mut self, ctx: &mut Context, e: &GameEvent) {
        if let GameEvent::Resized { .. } = e {
            self.view = View2D::fit(ctx.gfx.size(), self.layout.device);
        }
    }

    fn raw_window_event(&mut self, _ctx: &mut Context, e: &WindowEvent) -> bool {
        self.controls.window_event(e, &self.view, &self.layout);
        false
    }
}

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    run::<NuPogodi>(AppConfig {
        title: "Ну, погоди!".into(),
        width: WIDTH,
        height: HEIGHT,
        fixed_timestep: Some(Duration::from_secs_f64(1.0 / 120.0)),
        asset_root: platform::asset_root(asset_root!()),
        canvas_id: Some(platform::CANVAS_ID.into()),
        random_seed: u64::from(platform::seconds_of_day()).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        ..Default::default()
    })
}
