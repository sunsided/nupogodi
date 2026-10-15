//! Draws the game's art: the LCD segment sprites, the printed LCD background, the glass
//! overlay and the device bezel, and writes `assets/layout.json` (where each segment sits on
//! the LCD and where the buttons are on the bezel).
//!
//! The look follows the Elektronika IM-02: grey shell, recessed silver panel, red
//! "НУ ,ПОГОДИ!" title, red buttons, and an LCD film with red-brown henhouses and green bushes.
//!
//! Existing files are left alone so hand-drawn art is never overwritten; pass `--force` to
//! regenerate everything. Run it in release mode, the rasterizer supersamples every pixel.
//!
//! ```sh
//! cargo run --release -p gen-placeholders [-- --force]
//! ```

mod bezel;
mod canvas;
mod lcd;
mod text;

use std::path::{Path, PathBuf};

use image::RgbaImage;
use serde_json::json;

fn main() {
    let force = std::env::args().any(|a| a == "--force");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let segs = root.join("art/segments");
    std::fs::create_dir_all(&segs).expect("create art/segments");

    let sheet = lcd::segments();
    for (name, img) in &sheet.sprites {
        write_png(&segs.join(format!("{name}.png")), img, force);
    }
    for (name, img) in bezel::press_masks() {
        write_png(&segs.join(format!("{name}.png")), &img, force);
    }
    write_png(&root.join("art/lcd_bg.png"), &lcd::background(), force);
    write_png(&root.join("art/lcd_glass.png"), &lcd::glass(), force);
    write_png(&root.join("art/bezel.png"), &bezel::bezel(), force);

    let doc = json!({
        "device": [bezel::W, bezel::H],
        "lcd_origin": bezel::LCD_ORIGIN,
        "lcd_size": [lcd::W, lcd::H],
        "segments": sheet.layout,
        "buttons": bezel::buttons(),
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
