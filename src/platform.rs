//! The few things that differ between native and the browser.

/// The canvas the web build renders into.
pub const CANVAS_ID: &str = "nupogodi";

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::PathBuf;

    use chrono::Timelike;

    pub fn seconds_of_day() -> u32 {
        chrono::Local::now().num_seconds_from_midnight()
    }

    pub fn asset_root(default: PathBuf) -> PathBuf {
        default
    }

    pub fn is_active() -> bool {
        true
    }

    fn hiscore_path() -> Option<PathBuf> {
        Some(dirs::data_dir()?.join("nupogodi").join("hiscore"))
    }

    pub fn load_hiscore() -> u32 {
        hiscore_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    pub fn save_hiscore(score: u32) {
        let Some(p) = hiscore_path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&p, score.to_string()) {
            log::warn!("save hiscore to {}: {e}", p.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::path::PathBuf;

    const HISCORE_KEY: &str = "nupogodi.hiscore";

    pub fn seconds_of_day() -> u32 {
        let d = js_sys::Date::new_0();
        d.get_hours() * 3600 + d.get_minutes() * 60 + d.get_seconds()
    }

    fn canvas() -> Option<web_sys::Element> {
        web_sys::window()?
            .document()?
            .get_element_by_id(super::CANVAS_ID)
    }

    /// The host page can put the assets anywhere: `<canvas data-assets="/games/nupogodi/assets">`.
    pub fn asset_root(default: PathBuf) -> PathBuf {
        canvas()
            .and_then(|c| c.get_attribute("data-assets"))
            .map(PathBuf::from)
            .unwrap_or(default)
    }

    /// Paused while the canvas is detached from the page (the host component unmounted).
    pub fn is_active() -> bool {
        canvas().is_some_and(|c| c.is_connected())
    }

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load_hiscore() -> u32 {
        storage()
            .and_then(|s| s.get_item(HISCORE_KEY).ok()?)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    pub fn save_hiscore(score: u32) {
        if let Some(s) = storage() {
            let _ = s.set_item(HISCORE_KEY, &score.to_string());
        }
    }
}

pub use imp::*;
