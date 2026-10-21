//! The game's colours and the day/night blend between them.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn from_bytes(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
        }
    }

    fn inverted(self) -> Self {
        Self {
            r: 1.0 - self.r,
            g: 1.0 - self.g,
            b: 1.0 - self.b,
        }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub background: Rgb,
    /// The rex, obstacles, ground and score.
    pub foreground: Rgb,
    /// Secondary text such as the high score.
    pub faint: Rgb,
    /// The rex itself, so it stands out from the obstacles.
    pub accent: Rgb,
}

/// Chrome's own colours: a light grey on white.
pub const CHROME: Palette = Palette {
    background: Rgb::from_bytes(0xff, 0xff, 0xff),
    foreground: Rgb::from_bytes(0x53, 0x53, 0x53),
    faint: Rgb::from_bytes(0x75, 0x75, 0x75),
    accent: Rgb::from_bytes(0x53, 0x53, 0x53),
};

impl Palette {
    /// Blends towards the inverted palette, which is how Chrome shows night.
    /// `night` runs from 0 (day) to 1 (full night).
    pub fn at_night(self, night: f32) -> Self {
        let t = night.clamp(0.0, 1.0);
        Self {
            background: self.background.lerp(self.background.inverted(), t),
            foreground: self.foreground.lerp(self.foreground.inverted(), t),
            faint: self.faint.lerp(self.faint.inverted(), t),
            accent: self.accent.lerp(self.accent.inverted(), t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_is_unchanged() {
        assert_eq!(CHROME.at_night(0.0), CHROME);
    }

    fn close(a: Rgb, b: Rgb) -> bool {
        (a.r - b.r).abs() < 1e-6 && (a.g - b.g).abs() < 1e-6 && (a.b - b.b).abs() < 1e-6
    }

    #[test]
    fn full_night_inverts() {
        let night = CHROME.at_night(1.0);
        assert!(close(night.background, Rgb::from_bytes(0, 0, 0)));
        assert!(close(night.foreground, Rgb::from_bytes(0xac, 0xac, 0xac)));
    }

    #[test]
    fn dusk_is_halfway() {
        let dusk = CHROME.at_night(0.5);
        assert!((dusk.background.r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn out_of_range_is_clamped() {
        assert_eq!(CHROME.at_night(2.0), CHROME.at_night(1.0));
        assert_eq!(CHROME.at_night(-1.0), CHROME);
    }
}
