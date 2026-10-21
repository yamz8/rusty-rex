//! Fits the game's fixed playfield into whatever window it gets.
//!
//! Hyprland tiles new windows, so the game rarely gets the 960x320 it asks
//! for. Everything is drawn in playfield units and this works out which part
//! of that space the window shows.

pub const PLAYFIELD_WIDTH: f32 = 960.0;
pub const PLAYFIELD_HEIGHT: f32 = 320.0;
/// The widest stretch of playfield ever shown. Obstacles appear at x = 1200
/// and are removed at x = -300, so showing more on a very wide window would
/// let them pop in and out in plain sight. 1440 centred spans -240 to 1200.
pub const MAX_SHOWN_WIDTH: f32 = 1440.0;

/// A rectangle in playfield units: x, y, width, height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The playfield area a window of this size should show.
///
/// The playfield is scaled as large as it fits without cropping and kept
/// centred, so a window with a different shape shows extra space around it
/// instead of stretching the rex.
pub fn visible_area(window_width: f32, window_height: f32) -> Rect {
    // A minimised or not-yet-sized window can report zero, which would make
    // every coordinate infinite.
    if !(window_width > 0.0 && window_height > 0.0) {
        return Rect {
            x: 0.0,
            y: 0.0,
            w: PLAYFIELD_WIDTH,
            h: PLAYFIELD_HEIGHT,
        };
    }
    let scale = (window_width / PLAYFIELD_WIDTH).min(window_height / PLAYFIELD_HEIGHT);
    let w = window_width / scale;
    let h = window_height / scale;
    Rect {
        x: (PLAYFIELD_WIDTH - w) / 2.0,
        y: (PLAYFIELD_HEIGHT - h) / 2.0,
        w,
        h,
    }
}

/// The part of `area` that is actually drawn: all of it, unless the window
/// is so wide that it would show past MAX_SHOWN_WIDTH.
pub fn shown(area: Rect) -> Rect {
    if area.w <= MAX_SHOWN_WIDTH {
        return area;
    }
    Rect {
        x: (PLAYFIELD_WIDTH - MAX_SHOWN_WIDTH) / 2.0,
        w: MAX_SHOWN_WIDTH,
        ..area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn exact_size_shows_the_playfield() {
        let area = visible_area(960.0, 320.0);
        assert_eq!(
            area,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 960.0,
                h: 320.0
            }
        );
    }

    #[test]
    fn a_tall_window_adds_space_above_and_below() {
        let area = visible_area(1342.0, 718.0);
        assert!(close(area.w, 960.0));
        assert!(close(area.x, 0.0));
        assert!(area.h > 320.0);
        assert!(close(area.y, (320.0 - area.h) / 2.0));
    }

    #[test]
    fn a_wide_window_adds_space_at_the_sides() {
        let area = visible_area(2000.0, 320.0);
        assert!(close(area.h, 320.0));
        assert!(area.w > 960.0);
        assert!(close(area.x, (960.0 - area.w) / 2.0));
    }

    #[test]
    fn keeps_the_window_shape() {
        let area = visible_area(1342.0, 718.0);
        assert!(close(area.w / area.h, 1342.0 / 718.0));
    }

    #[test]
    fn a_zero_sized_window_shows_the_plain_playfield() {
        for (w, h) in [(0.0, 0.0), (1342.0, 0.0), (0.0, 718.0), (f32::NAN, 10.0)] {
            let area = visible_area(w, h);
            assert_eq!(area.w, PLAYFIELD_WIDTH, "{w}x{h}");
            assert!(area.x.is_finite() && area.y.is_finite() && area.h.is_finite());
        }
    }

    #[test]
    fn ultrawide_windows_never_show_obstacles_appearing_or_vanishing() {
        use crate::game::{DESPAWN_X, SPAWN_X};
        for (w, h) in [
            (1920.0, 400.0),
            (2560.0, 500.0),
            (3440.0, 600.0),
            (5000.0, 300.0),
        ] {
            let s = shown(visible_area(w, h));
            assert!(s.x >= DESPAWN_X, "{w}x{h}: left {}", s.x);
            assert!(s.x + s.w <= SPAWN_X, "{w}x{h}: right {}", s.x + s.w);
        }
    }

    #[test]
    fn ordinary_windows_are_not_cropped() {
        let area = visible_area(1342.0, 718.0);
        assert_eq!(shown(area), area);
    }
}
