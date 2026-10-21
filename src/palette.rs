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

/// At night a dark theme's background is pulled this far towards black.
const DARK_NIGHT_SKY: f32 = 0.6;
/// At night a dark theme's foreground colours sink this far into the sky.
const DARK_NIGHT_DIM: f32 = 0.35;
const BLACK: Rgb = Rgb::from_bytes(0, 0, 0);

/// Chrome's own colours: a light grey on white.
pub const CHROME: Palette = Palette {
    background: Rgb::from_bytes(0xff, 0xff, 0xff),
    foreground: Rgb::from_bytes(0x53, 0x53, 0x53),
    faint: Rgb::from_bytes(0x75, 0x75, 0x75),
    accent: Rgb::from_bytes(0x53, 0x53, 0x53),
};

impl Palette {
    /// Uses the same test as Omarchy's own theme resolver: a background
    /// whose channels average over half is light.
    pub fn is_dark(&self) -> bool {
        let b = self.background;
        (b.r + b.g + b.b) / 3.0 <= 0.5
    }

    /// The palette at full night. Light themes invert, as Chrome does. Dark
    /// themes would invert into a bright screen, so they get darker instead.
    fn night(self) -> Self {
        if self.is_dark() {
            let sky = self.background.lerp(BLACK, DARK_NIGHT_SKY);
            Self {
                background: sky,
                foreground: self.foreground.lerp(sky, DARK_NIGHT_DIM),
                faint: self.faint.lerp(sky, DARK_NIGHT_DIM),
                accent: self.accent.lerp(sky, DARK_NIGHT_DIM),
            }
        } else {
            Self {
                background: self.background.inverted(),
                foreground: self.foreground.inverted(),
                faint: self.faint.inverted(),
                accent: self.accent.inverted(),
            }
        }
    }

    /// Blends from day towards night. `night` runs from 0 (day) to 1 (full
    /// night).
    pub fn at_night(self, night: f32) -> Self {
        let t = night.clamp(0.0, 1.0);
        let n = self.night();
        Self {
            background: self.background.lerp(n.background, t),
            foreground: self.foreground.lerp(n.foreground, t),
            faint: self.faint.lerp(n.faint, t),
            accent: self.accent.lerp(n.accent, t),
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
    fn a_light_theme_inverts_at_night() {
        let night = CHROME.at_night(1.0);
        assert!(close(night.background, Rgb::from_bytes(0, 0, 0)));
        assert!(close(night.foreground, Rgb::from_bytes(0xac, 0xac, 0xac)));
    }

    #[test]
    fn dusk_is_halfway() {
        let dusk = CHROME.at_night(0.5);
        assert!((dusk.background.r - 0.5).abs() < 1e-6);
    }

    const MATTE_BLACK: Palette = Palette {
        background: Rgb::from_bytes(0x12, 0x12, 0x12),
        foreground: Rgb::from_bytes(0xbe, 0xbe, 0xbe),
        faint: Rgb::from_bytes(0x80, 0x80, 0x80),
        accent: Rgb::from_bytes(0xe6, 0x8e, 0x0d),
    };

    fn brightness(c: Rgb) -> f32 {
        (c.r + c.g + c.b) / 3.0
    }

    #[test]
    fn tells_light_from_dark() {
        assert!(!CHROME.is_dark());
        assert!(MATTE_BLACK.is_dark());
    }

    #[test]
    fn a_dark_theme_gets_darker_at_night() {
        let night = MATTE_BLACK.at_night(1.0);
        assert!(brightness(night.background) < brightness(MATTE_BLACK.background));
        assert!(brightness(night.foreground) < brightness(MATTE_BLACK.foreground));
        // Still readable: the rex and obstacles stay well above the sky.
        assert!(brightness(night.foreground) - brightness(night.background) > 0.3);
        assert!(brightness(night.accent) > brightness(night.background));
    }

    #[test]
    fn out_of_range_is_clamped() {
        assert_eq!(CHROME.at_night(2.0), CHROME.at_night(1.0));
        assert_eq!(CHROME.at_night(-1.0), CHROME);
    }
}
