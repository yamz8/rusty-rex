//! Reads the current Omarchy theme's colours.
//!
//! Omarchy keeps the active theme's palette in
//! `~/.local/state/omarchy/current/theme/colors.toml` as simple
//! `key = "#rrggbb"` lines. Only the few keys the game uses are resolved, with
//! the same fallbacks Omarchy's own resolver (`omarchy-theme-color`) applies.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::palette::{Palette, Rgb};

/// How much of the background is mixed into the foreground for faint text.
const FAINT_MIX: f32 = 0.35;

pub fn colors_path(home: Option<OsString>) -> Option<PathBuf> {
    Some(PathBuf::from(home?).join(".local/state/omarchy/current/theme/colors.toml"))
}

/// Builds a palette from a colors.toml, or None if it lacks a usable
/// background or foreground.
pub fn parse(text: &str) -> Option<Palette> {
    let values = read_pairs(text);
    let pick = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| values.get(*k).and_then(|v| hex(v)))
    };

    // Older themes use short names or only the ANSI colours.
    let background = pick(&["background", "bg", "color0"])?;
    let foreground = pick(&["foreground", "fg", "color7"])?;
    let accent = pick(&["accent"]).unwrap_or(foreground);
    Some(Palette {
        background,
        foreground,
        faint: foreground.lerp(background, FAINT_MIX),
        accent,
    })
}

/// Splits `key = value` lines, dropping comments and quotes the way Omarchy's
/// parser does.
fn read_pairs(text: &str) -> HashMap<String, String> {
    let mut pairs = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches(['"', '\'']);
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        let value = value.trim();
        let value = match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
            // An unquoted value ends at an inline comment.
            _ => value.split(" #").next().unwrap_or("").trim(),
        };
        pairs.insert(key.to_owned(), value.to_owned());
    }
    pairs
}

/// Parses `#rrggbb`; anything else (rgb() lists, short forms) is skipped.
fn hex(value: &str) -> Option<Rgb> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
    Some(Rgb::from_bytes(byte(0)?, byte(2)?, byte(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATTE_BLACK: &str = r##"mode = "dark"

accent = "#e68e0d"
muted = "#333333"

background = "#121212"
foreground = "#bebebe"
red = "#D35F5F"
"##;

    #[test]
    fn reads_an_omarchy_theme() {
        let p = parse(MATTE_BLACK).unwrap();
        assert_eq!(p.background, Rgb::from_bytes(0x12, 0x12, 0x12));
        assert_eq!(p.foreground, Rgb::from_bytes(0xbe, 0xbe, 0xbe));
        assert_eq!(p.accent, Rgb::from_bytes(0xe6, 0x8e, 0x0d));
    }

    #[test]
    fn faint_sits_between_foreground_and_background() {
        let p = parse(MATTE_BLACK).unwrap();
        assert!(p.faint.r < p.foreground.r && p.faint.r > p.background.r);
    }

    #[test]
    fn falls_back_to_short_and_ansi_names() {
        let short = parse("bg = \"#000000\"\nfg = \"#ffffff\"").unwrap();
        assert_eq!(short.background, Rgb::from_bytes(0, 0, 0));
        let ansi = parse("color0 = '#101010'\ncolor7 = '#efefef'").unwrap();
        assert_eq!(ansi.foreground, Rgb::from_bytes(0xef, 0xef, 0xef));
        // No accent means the rex just uses the foreground.
        assert_eq!(ansi.accent, ansi.foreground);
    }

    #[test]
    fn canonical_names_win_over_legacy_ones() {
        let p = parse("bg = \"#000000\"\nbackground = \"#222222\"\nfg = \"#ffffff\"").unwrap();
        assert_eq!(p.background, Rgb::from_bytes(0x22, 0x22, 0x22));
    }

    #[test]
    fn ignores_comments_and_unusable_values() {
        let text = "# background = \"#ff0000\"\n\
                    background = #202020 # inline comment\n\
                    accent = \"rgb(1, 2, 3)\"\n\
                    foreground = \"#dddddd\" # trailing";
        let p = parse(text).unwrap();
        assert_eq!(p.background, Rgb::from_bytes(0x20, 0x20, 0x20));
        assert_eq!(p.accent, p.foreground);
    }

    #[test]
    fn incomplete_themes_are_rejected() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("background = \"#121212\""), None);
        assert_eq!(
            parse("background = \"#12\"\nforeground = \"#bebebe\""),
            None
        );
    }

    #[test]
    fn finds_the_current_theme() {
        assert_eq!(
            colors_path(Some("/home/rex".into())),
            Some(PathBuf::from(
                "/home/rex/.local/state/omarchy/current/theme/colors.toml"
            ))
        );
        assert_eq!(colors_path(None), None);
    }
}
