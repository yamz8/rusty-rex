//! Fits the game's fixed playfield into whatever window it gets.
//!
//! Hyprland tiles new windows, so the game rarely gets the 960x320 it asks
//! for. Everything is drawn in playfield units and this works out which part
//! of that space the window shows.

pub const PLAYFIELD_WIDTH: f32 = 960.0;
pub const PLAYFIELD_HEIGHT: f32 = 320.0;

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
}
