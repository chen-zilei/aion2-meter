//! Remembers where the overlay sits on screen (position and size), so it comes back there after a restart.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Monitor, PhysicalPosition, PhysicalSize, WebviewWindow, Window};

/// How much of the overlay's top edge must be on some screen to restore it there: enough to grab and drag it.
const MIN_VISIBLE_W: i64 = 60;
const MIN_VISIBLE_H: i64 = 24;
/// Wait this long after the last move or resize before writing, so a drag is one write, not hundreds.
const SAVE_DELAY_MS: u64 = 500;

/// Window bounds in physical pixels, as the OS reports them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub struct OverlayPlace {
    path: PathBuf,
    /// Bounds waiting to be written, with the time of the change that produced them.
    pending: Mutex<Option<(Bounds, u64)>>,
}

impl OverlayPlace {
    pub fn new(path: PathBuf) -> Self {
        Self { path, pending: Mutex::new(None) }
    }

    /// Puts the overlay back where it was, unless that spot is no longer on any screen (say, a monitor was unplugged),
    /// in which case it stays at its default spot.
    pub fn restore(&self, window: &WebviewWindow) {
        let Some(b) = std::fs::read_to_string(&self.path).ok().and_then(|s| serde_json::from_str::<Bounds>(&s).ok()) else {
            return;
        };
        let monitors = window.available_monitors().unwrap_or_default();
        if !on_screen(&b, &monitors) {
            return;
        }
        let _ = window.set_size(PhysicalSize::new(b.w, b.h));
        let _ = window.set_position(PhysicalPosition::new(b.x, b.y));
    }

    /// Notes the overlay's current bounds after it moved or was resized; `flush` writes them once it settles.
    pub fn changed(&self, window: &Window, now_ms: u64) {
        // A minimised window reports a far-off placeholder position; don't remember that.
        if window.is_minimized().unwrap_or(false) {
            return;
        }
        let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) else { return };
        *self.pending.lock() = Some((Bounds { x: pos.x, y: pos.y, w: size.width, h: size.height }, now_ms));
    }

    /// Writes pending bounds once no move or resize has happened for a moment. Call periodically.
    pub fn flush(&self, now_ms: u64) {
        let mut pending = self.pending.lock();
        let Some((b, at)) = *pending else { return };
        if now_ms.saturating_sub(at) < SAVE_DELAY_MS {
            return;
        }
        *pending = None;
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&b) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

fn on_screen(b: &Bounds, monitors: &[Monitor]) -> bool {
    let rects: Vec<(i64, i64, i64, i64)> = monitors
        .iter()
        .map(|m| (m.position().x as i64, m.position().y as i64, m.size().width as i64, m.size().height as i64))
        .collect();
    top_strip_visible(b, &rects)
}

/// Whether a grabbable piece of the window's top strip lies on one of the screens (`x, y, w, h` each).
fn top_strip_visible(b: &Bounds, screens: &[(i64, i64, i64, i64)]) -> bool {
    let (x0, y0) = (b.x as i64, b.y as i64);
    let (x1, y1) = (x0 + b.w as i64, y0 + MIN_VISIBLE_H);
    screens.iter().any(|&(sx, sy, sw, sh)| {
        let w = x1.min(sx + sw) - x0.max(sx);
        let h = y1.min(sy + sh) - y0.max(sy);
        w >= MIN_VISIBLE_W && h >= MIN_VISIBLE_H
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: (i64, i64, i64, i64) = (0, 0, 1920, 1080);
    const LEFT: (i64, i64, i64, i64) = (-2560, 0, 2560, 1440);

    fn at(x: i32, y: i32) -> Bounds {
        Bounds { x, y, w: 340, h: 280 }
    }

    #[test]
    fn restores_a_spot_on_any_screen() {
        assert!(top_strip_visible(&at(40, 200), &[MAIN]));
        assert!(top_strip_visible(&at(-1200, 300), &[MAIN, LEFT]));
        // Mostly off the right edge, but the left part of the title strip is still grabbable.
        assert!(top_strip_visible(&at(1800, 500), &[MAIN]));
    }

    #[test]
    fn skips_a_spot_on_a_screen_that_is_gone() {
        assert!(!top_strip_visible(&at(-1200, 300), &[MAIN]));
        assert!(!top_strip_visible(&at(1900, 500), &[MAIN]));
        // Title strip above the top of the screen: it couldn't be dragged back.
        assert!(!top_strip_visible(&at(40, -100), &[MAIN]));
        assert!(!top_strip_visible(&at(40, 200), &[]));
    }
}
