//! The window's size (ADR 0008, ARCHITECTURE §11): 1440 × 900 if it fits the monitor, else
//! 1280 × 800 showing the same 1440 × 900 layout zoomed to 8/9.

/// What the window opens at: its content size in logical pixels, and the webview zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowFit {
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
}

pub const FULL: WindowFit = WindowFit {
    width: 1440.0,
    height: 900.0,
    zoom: 1.0,
};

/// At 8/9 the page still measures 1440 × 900 CSS pixels, so there's no second layout.
pub const FALLBACK: WindowFit = WindowFit {
    width: 1280.0,
    height: 800.0,
    zoom: 8.0 / 9.0,
};

/// `work_area` and `frame` (the title bar and borders around the content) are in physical
/// pixels; `scale` is the monitor's display scaling (1.25 at 125%).
pub fn window_fit(work_area: (u32, u32), frame: (u32, u32), scale: f64) -> WindowFit {
    let needed = |logical: f64, frame: u32| (logical * scale).ceil() + f64::from(frame);
    if needed(FULL.width, frame.0) <= f64::from(work_area.0)
        && needed(FULL.height, frame.1) <= f64::from(work_area.1)
    {
        FULL
    } else {
        FALLBACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Windows 11's title bar and borders at 100%: about 16 × 39 physical pixels.
    const FRAME: (u32, u32) = (16, 39);

    #[test]
    fn the_owners_monitors_at_100_percent_get_the_full_size() {
        // 1920 × 1080 and 2560 × 1080, each with about 1032 px above the taskbar (ADR 0008).
        assert_eq!(window_fit((1920, 1032), FRAME, 1.0), FULL);
        assert_eq!(window_fit((2560, 1032), FRAME, 1.0), FULL);
    }

    #[test]
    fn exactly_enough_room_still_fits() {
        assert_eq!(window_fit((1456, 939), FRAME, 1.0), FULL);
        assert_eq!(window_fit((1456, 938), FRAME, 1.0), FALLBACK);
        assert_eq!(window_fit((1455, 939), FRAME, 1.0), FALLBACK);
    }

    #[test]
    fn raised_display_scaling_falls_back() {
        // 1080p at 125%: 1125 px of 1350 logical would be needed; only ~1032 is there.
        assert_eq!(window_fit((1920, 1032), (20, 49), 1.25), FALLBACK);
    }

    #[test]
    fn a_small_screen_falls_back() {
        assert_eq!(window_fit((1366, 728), FRAME, 1.0), FALLBACK);
    }

    #[test]
    fn the_fallback_shows_the_same_layout() {
        assert!((FALLBACK.width / FALLBACK.zoom - FULL.width).abs() < 1e-9);
        assert!((FALLBACK.height / FALLBACK.zoom - FULL.height).abs() < 1e-9);
    }
}
