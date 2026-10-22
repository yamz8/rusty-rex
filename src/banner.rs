//! Turns half-block text art into a pixel grid for drawing in the sky.
//!
//! The game's font has no block glyphs, so the art is read as pixels: each
//! character is one column wide and two pixels tall, and the half blocks say
//! which of the two are lit. Anything else is a blank cell, so stray text in
//! the file can't turn into unreadable smudges.

use crate::sprites::Sprite;

/// Builds a sprite from the art, or None if it has no block pixels at all.
pub fn parse(text: &str) -> Option<Sprite> {
    let mut grid: Vec<Vec<bool>> = Vec::new();
    for line in text.lines() {
        let (top, bottom): (Vec<bool>, Vec<bool>) = line
            .chars()
            .map(|c| match c {
                '█' => (true, true),
                '▀' => (true, false),
                '▄' => (false, true),
                _ => (false, false),
            })
            .unzip();
        grid.push(top);
        grid.push(bottom);
    }

    // Trim to the lit pixels, so the art centres on what is drawn rather
    // than on the file's padding.
    let lit = |row: &Vec<bool>| row.iter().any(|&p| p);
    let first_row = grid.iter().position(lit)?;
    let last_row = grid.iter().rposition(lit)?;
    let grid = &grid[first_row..=last_row];
    let first_col = grid
        .iter()
        .filter_map(|row| row.iter().position(|&p| p))
        .min()?;
    let last_col = grid
        .iter()
        .filter_map(|row| row.iter().rposition(|&p| p))
        .max()?;

    let rows: Vec<String> = grid
        .iter()
        .map(|row| {
            (first_col..=last_col)
                .map(|x| if row.get(x) == Some(&true) { '#' } else { '.' })
                .collect()
        })
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    Some(Sprite::new(&rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lit pixels as `#`/`.` rows, for comparing against expected art.
    fn pixels(sprite: &Sprite) -> Vec<String> {
        let mut rows = vec![vec!['.'; sprite.cols]; sprite.rows];
        for run in &sprite.runs {
            for row in rows.iter_mut().skip(run.y).take(run.height) {
                for cell in row.iter_mut().skip(run.x).take(run.len) {
                    *cell = '#';
                }
            }
        }
        rows.into_iter().map(|r| r.into_iter().collect()).collect()
    }

    #[test]
    fn half_blocks_light_the_top_bottom_or_both_pixels() {
        let art = parse("█▀▄").unwrap();
        assert_eq!(pixels(&art), ["##.", "#.#"]);
    }

    #[test]
    fn other_characters_are_blank() {
        let art = parse("█a█\n# ▀").unwrap();
        assert_eq!(pixels(&art), ["#.#", "#.#", "..#"]);
    }

    #[test]
    fn padding_around_the_art_is_trimmed() {
        let art = parse("\n   \n  ▄▄ \n  ▀▀\n\n").unwrap();
        assert_eq!((art.cols, art.rows), (2, 2));
        assert_eq!(pixels(&art), ["##", "##"]);
    }

    #[test]
    fn short_lines_are_blank_to_the_right() {
        let art = parse("███\n█").unwrap();
        assert_eq!(pixels(&art), ["###", "###", "#..", "#.."]);
    }

    #[test]
    fn nothing_to_draw_is_none() {
        assert!(parse("").is_none());
        assert!(parse("plain text\n  \n").is_none());
    }

    #[test]
    fn a_screensaver_logo_keeps_its_shape() {
        // Same shape as Omarchy's 81-column, 10-line logo: a blank margin
        // and a full-width body, with the first line holding the top edge.
        let mut text = " ".repeat(17) + "▄▄▄\n";
        for _ in 0..8 {
            text += &(" ".to_string() + &"█".repeat(80) + "\n");
        }
        text += &(" ".repeat(39) + "███   █▀\n");
        let art = parse(&text).unwrap();
        assert_eq!((art.cols, art.rows), (80, 19));
    }
}
