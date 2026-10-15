//! Keyboard, mouse and touch → device buttons.

use std::collections::HashMap;

use game_toolkit::prelude::*;
use winit::event::{TouchPhase, WindowEvent};

use crate::layout::Layout;
use crate::logic::{Inputs, Ramp};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Ramp(Ramp),
    GameA,
    GameB,
    Time,
    AlarmSet,
    TimeSet,
}

impl Button {
    pub const ALL: [Button; 9] = [
        Button::Ramp(Ramp::TopLeft),
        Button::Ramp(Ramp::BottomLeft),
        Button::Ramp(Ramp::TopRight),
        Button::Ramp(Ramp::BottomRight),
        Button::GameA,
        Button::GameB,
        Button::Time,
        Button::AlarmSet,
        Button::TimeSet,
    ];

    /// Key in `layout.json`'s `buttons`.
    pub fn name(self) -> &'static str {
        match self {
            Button::Ramp(Ramp::TopLeft) => "ramp_tl",
            Button::Ramp(Ramp::BottomLeft) => "ramp_bl",
            Button::Ramp(Ramp::TopRight) => "ramp_tr",
            Button::Ramp(Ramp::BottomRight) => "ramp_br",
            Button::GameA => "game_a",
            Button::GameB => "game_b",
            Button::Time => "time",
            Button::AlarmSet => "alarm_set",
            Button::TimeSet => "time_set",
        }
    }

    fn keys(self) -> &'static [Key] {
        match self {
            Button::Ramp(Ramp::TopLeft) => &[Key::KeyQ, Key::Numpad7, Key::KeyW],
            Button::Ramp(Ramp::BottomLeft) => &[Key::KeyA, Key::Numpad1, Key::KeyS],
            Button::Ramp(Ramp::TopRight) => &[Key::KeyP, Key::Numpad9, Key::KeyO],
            Button::Ramp(Ramp::BottomRight) => &[Key::KeyL, Key::Numpad3, Key::KeyK],
            Button::GameA => &[Key::Digit1],
            Button::GameB => &[Key::Digit2],
            Button::Time => &[Key::KeyT, Key::Digit3],
            Button::AlarmSet => &[Key::KeyZ],
            Button::TimeSet => &[Key::KeyX],
        }
    }
}

/// Maps between window pixels and device space (the bezel's logical pixels).
#[derive(Copy, Clone, Debug)]
pub struct View2D {
    pub zoom: f32,
    /// Device-space point at the window's top-left.
    pub top_left: [f32; 2],
}

impl View2D {
    pub fn fit(window: (u32, u32), device: [f32; 2]) -> Self {
        let (w, h) = (window.0.max(1) as f32, window.1.max(1) as f32);
        let zoom = (w / device[0]).min(h / device[1]);
        Self {
            zoom,
            top_left: [
                device[0] * 0.5 - w * 0.5 / zoom,
                device[1] * 0.5 - h * 0.5 / zoom,
            ],
        }
    }

    pub fn to_device(self, p: [f32; 2]) -> [f32; 2] {
        [
            p[0] / self.zoom + self.top_left[0],
            p[1] / self.zoom + self.top_left[1],
        ]
    }
}

#[derive(Default)]
pub struct Controls {
    /// Buttons held by the mouse (`None` key) or by a touch id.
    pointers: HashMap<Option<u64>, Button>,
    /// Pointer presses since the last `poll`.
    pressed: Vec<Button>,
    held_last: Vec<Button>,
}

impl Controls {
    fn hit(layout: &Layout, p: [f32; 2]) -> Option<Button> {
        Button::ALL
            .into_iter()
            .find(|b| layout.buttons.get(b.name()).is_some_and(|s| s.contains(p)))
    }

    fn pointer_down(&mut self, id: Option<u64>, b: Option<Button>) {
        if let Some(b) = b {
            self.pointers.insert(id, b);
            self.pressed.push(b);
        }
    }

    /// Feed raw window events; touches don't reach the toolkit's `Input`.
    pub fn window_event(&mut self, e: &WindowEvent, view: &View2D, layout: &Layout) {
        if let WindowEvent::Touch(t) = e {
            let p = view.to_device([t.location.x as f32, t.location.y as f32]);
            match t.phase {
                TouchPhase::Started => self.pointer_down(Some(t.id), Self::hit(layout, p)),
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.pointers.remove(&Some(t.id));
                }
                TouchPhase::Moved => {}
            }
        }
    }

    /// Collect this update's button presses.
    pub fn poll(&mut self, input: &Input, view: &View2D, layout: &Layout) -> Inputs {
        let (mx, my) = input.mouse_pos();
        if input.mouse_pressed(MouseButton::Left) {
            let b = Self::hit(layout, view.to_device([mx, my]));
            self.pointer_down(None, b);
        }
        if !input.mouse_held(MouseButton::Left) {
            self.pointers.remove(&None);
        }

        let mut pressed = std::mem::take(&mut self.pressed);
        for b in Button::ALL {
            if b.keys().iter().any(|&k| input.key_pressed(k)) {
                pressed.push(b);
            }
        }
        let held = self.held();
        self.held_last = held.clone();

        let mut out = Inputs {
            time_held: held.contains(&Button::Time),
            ..Default::default()
        };
        for b in pressed {
            match b {
                Button::Ramp(r) => out.ramp = Some(r),
                Button::GameA => out.game_a = true,
                Button::GameB => out.game_b = true,
                Button::Time => out.time = true,
                Button::AlarmSet => out.alarm_set = true,
                Button::TimeSet => out.time_set = true,
            }
        }
        out.time_held |= input_held(input, Button::Time);
        out
    }

    fn held(&self) -> Vec<Button> {
        self.pointers.values().copied().collect()
    }

    /// Buttons to draw pressed: pointer-held ones plus held keys.
    pub fn visually_held(&self, input: &Input) -> Vec<Button> {
        let mut v = self.held_last.clone();
        v.extend(Button::ALL.into_iter().filter(|&b| input_held(input, b)));
        v
    }
}

fn input_held(input: &Input, b: Button) -> bool {
    b.keys().iter().any(|&k| input.key_held(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    impl View2D {
        fn to_screen(self, p: [f32; 2]) -> [f32; 2] {
            [
                (p[0] - self.top_left[0]) * self.zoom,
                (p[1] - self.top_left[1]) * self.zoom,
            ]
        }
    }

    #[test]
    fn fit_letterboxes_and_round_trips() {
        let v = View2D::fit((2240, 2000), [1120.0, 672.0]);
        assert_eq!(v.zoom, 2.0);
        let p = [100.0, 200.0];
        let s = v.to_screen(p);
        let back = v.to_device(s);
        assert!((back[0] - p[0]).abs() < 1e-3 && (back[1] - p[1]).abs() < 1e-3);
        // Centered vertically: the device's middle is the window's middle.
        assert_eq!(v.to_screen([560.0, 336.0]), [1120.0, 1000.0]);
    }
}
