//! The segment atlas packed by `atlas-packer`: one texture plus frame rects by name.

use std::collections::HashMap;

use game_toolkit::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    frames: HashMap<String, Entry>,
    meta: Meta,
}

#[derive(Deserialize)]
struct Entry {
    frame: Rect,
}

#[derive(Deserialize)]
struct Meta {
    size: Size,
}

#[derive(Deserialize)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Deserialize)]
struct Size {
    w: f32,
    h: f32,
}

/// A sub-image of the atlas.
#[derive(Copy, Clone, Debug)]
pub struct Sprite {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub size: [f32; 2],
}

pub struct Atlas {
    pub texture: TextureId,
    sprites: HashMap<String, Sprite>,
}

impl Atlas {
    pub fn load(ctx: &mut Context, png: &str, json: &str) -> Result<Self> {
        let texture = ctx.load_texture(png)?;
        let bytes = ctx.assets.read(json)?;
        Self::parse(texture, &bytes)
    }

    fn parse(texture: TextureId, bytes: &[u8]) -> Result<Self> {
        let file: File = serde_json::from_slice(bytes)?;
        let (aw, ah) = (file.meta.size.w, file.meta.size.h);
        let sprites = file
            .frames
            .into_iter()
            .map(|(name, e)| {
                let r = e.frame;
                let name = name.strip_suffix(".png").unwrap_or(&name).to_owned();
                let sprite = Sprite {
                    uv_min: [r.x / aw, r.y / ah],
                    uv_max: [(r.x + r.w) / aw, (r.y + r.h) / ah],
                    size: [r.w, r.h],
                };
                (name, sprite)
            })
            .collect();
        Ok(Self { texture, sprites })
    }

    pub fn get(&self, name: &str) -> Option<Sprite> {
        self.sprites.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atlas_packer_output() {
        let json = br#"{"frames":{"egg.png":{"frame":{"x":10,"y":20,"w":16,"h":12}}},
            "meta":{"size":{"w":100,"h":200}}}"#;
        let a = Atlas::parse(TextureId(0), json).unwrap();
        let s = a.get("egg").unwrap();
        assert_eq!(s.size, [16.0, 12.0]);
        assert_eq!(s.uv_min, [0.1, 0.1]);
        assert_eq!(s.uv_max, [0.26, 0.16]);
    }
}
