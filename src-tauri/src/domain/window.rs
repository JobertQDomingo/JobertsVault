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

/// `window` is the hidden window's outer size as created at 1440 × 900 (title bar, borders and
/// display scaling included) and `work_area` the monitor's usable area, both in physical pixels.
/// The fallback is used even where it doesn't fit either (the limit ADR 0008 accepts).
pub fn window_fit(window: (u32, u32), work_area: (u32, u32)) -> WindowFit {
    if window.0 <= work_area.0 && window.1 <= work_area.1 {
        FULL
    } else {
        FALLBACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1440 × 900 at 100% with Windows 11's title bar and borders.
    const AT_100: (u32, u32) = (1456, 939);

    #[test]
    fn the_owners_monitors_at_100_percent_get_the_full_size() {
        // 1920 × 1080 and 2560 × 1080, each with about 1032 px above the taskbar (ADR 0008).
        assert_eq!(window_fit(AT_100, (1920, 1032)), FULL);
        assert_eq!(window_fit(AT_100, (2560, 1032)), FULL);
    }

    #[test]
    fn exactly_enough_room_still_fits_and_one_pixel_less_does_not() {
        assert_eq!(window_fit(AT_100, (1456, 939)), FULL);
        assert_eq!(window_fit(AT_100, (1456, 938)), FALLBACK);
        assert_eq!(window_fit(AT_100, (1455, 939)), FALLBACK);
    }

    #[test]
    fn raised_display_scaling_falls_back() {
        // At 125% the same window is 1800 × 1125 + a 49 px title bar; 1080p leaves 1032.
        assert_eq!(window_fit((1820, 1174), (1920, 1032)), FALLBACK);
    }

    #[test]
    fn a_smaller_screen_falls_back() {
        // 1600 × 900: 939 px needed, 860 there; the fallback's 839 fits.
        assert_eq!(window_fit(AT_100, (1600, 860)), FALLBACK);
    }

    #[test]
    fn a_screen_too_small_even_for_the_fallback_still_gets_it() {
        // 1366 × 768: the accepted limit (ADR 0008), the smaller of the two sizes.
        assert_eq!(window_fit(AT_100, (1366, 728)), FALLBACK);
    }

    #[test]
    fn the_fallback_shows_the_same_layout() {
        assert!((FALLBACK.width / FALLBACK.zoom - FULL.width).abs() < 1e-9);
        assert!((FALLBACK.height / FALLBACK.zoom - FULL.height).abs() < 1e-9);
    }
}
