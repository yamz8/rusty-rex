//! Turns block and braille text art into a pixel grid for drawing in the
//! sky.
//!
//! The game's font has none of these glyphs, so the art is read as pixels:
//! each character is a cell two pixels wide and four tall, the braille dot
//! grid. Braille lights its dots, and the block characters (half blocks,
//! side halves and quadrants) light the quarters they fill, so art made of
//! either keeps its shape. Anything else is a blank cell, so stray text in
//! the file can't turn into unreadable smudges.

use crate::sprites::Sprite;

/// The bit for the pixel `[row][column]` of a cell, in braille's own dot
/// order, so a braille character's code is already its cell.
const DOT: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// The quarters of a cell, as dots.
const UPPER_LEFT: u8 = 0x01 | 0x02;
const UPPER_RIGHT: u8 = 0x08 | 0x10;
const LOWER_LEFT: u8 = 0x04 | 0x40;
const LOWER_RIGHT: u8 = 0x20 | 0x80;

/// Which of a character's eight pixels are lit.
fn cell(c: char) -> u8 {
    const UL: u8 = UPPER_LEFT;
    const UR: u8 = UPPER_RIGHT;
    const LL: u8 = LOWER_LEFT;
    const LR: u8 = LOWER_RIGHT;
    match c {
        '\u{2800}'..='\u{28ff}' => (c as u32 - 0x2800) as u8,
        '█' => UL | UR | LL | LR,
        '▀' => UL | UR,
        '▄' => LL | LR,
        '▌' => UL | LL,
        '▐' => UR | LR,
        '▘' => UL,
        '▝' => UR,
        '▖' => LL,
        '▗' => LR,
        '▚' => UL | LR,
        '▞' => UR | LL,
        '▙' => UL | LL | LR,
        '▛' => UL | UR | LL,
        '▜' => UL | UR | LR,
        '▟' => UR | LL | LR,
        _ => 0,
    }
}

/// Builds a sprite from the art, or None if it has nothing to draw.
pub fn parse(text: &str) -> Option<Sprite> {
    pixels(text).map(|grid| Sprite::from_pixels(&grid))
}

/// The art's lit pixels, row by row and trimmed to them, or None if it has
/// nothing to draw. Every row is as wide as the widest.
pub fn pixels(text: &str) -> Option<Vec<Vec<bool>>> {
    let mut grid: Vec<Vec<bool>> = Vec::new();
    for line in text.lines() {
        let cells: Vec<u8> = line.chars().map(cell).collect();
        for dots in DOT {
            grid.push(
                cells
                    .iter()
                    .flat_map(|&c| dots.map(|dot| c & dot != 0))
                    .collect(),
            );
        }
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
    Some(
        grid.iter()
            .map(|row| {
                (first_col..=last_col)
                    .map(|x| row.get(x) == Some(&true))
                    .collect()
            })
            .collect(),
    )
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
    fn half_blocks_light_the_top_bottom_or_both_halves() {
        let art = parse("█▀▄").unwrap();
        assert_eq!(pixels(&art), ["####..", "####..", "##..##", "##..##"]);
    }

    #[test]
    fn side_halves_and_quadrants_light_their_quarters() {
        assert_eq!(pixels(&parse("▌▐").unwrap()), ["#..#"; 4]);
        assert_eq!(pixels(&parse("▚").unwrap()), ["#.", "#.", ".#", ".#"]);
        assert_eq!(pixels(&parse("▟").unwrap()), [".#", ".#", "##", "##"]);
        assert_eq!(
            pixels(&parse("▘▗").unwrap()),
            ["#...", "#...", "...#", "...#"]
        );
    }

    #[test]
    fn braille_lights_its_dots() {
        // Dots 1, 2, 3 and 7 are the left column; 4, 5, 6 and 8 the right.
        assert_eq!(pixels(&parse("⡇⢸").unwrap()), ["#..#"; 4]);
        assert_eq!(pixels(&parse("⣿").unwrap()), ["##"; 4]);
        // Dot 1 alone, then dot 8 alone, trimmed to what is lit.
        assert_eq!(
            pixels(&parse("⠁⢀").unwrap()),
            ["#...", "....", "....", "...#"]
        );
        assert!(parse("\u{2800}").is_none(), "the blank braille cell");
    }

    #[test]
    fn half_block_art_keeps_its_shape() {
        // A half block is a quarter of a cell, so each old pixel is now a
        // 2x2 square and the proportions don't change.
        let art = parse("▀▄").unwrap();
        assert_eq!((art.cols, art.rows), (4, 4));
    }

    #[test]
    fn other_characters_are_blank() {
        let art = parse("█a█\n# ▀").unwrap();
        let mut want = vec!["##..##"; 4];
        want.extend(["....##"; 2]);
        assert_eq!(pixels(&art), want);
    }

    #[test]
    fn padding_around_the_art_is_trimmed() {
        let art = parse("\n   \n  ▄▄ \n  ▀▀\n\n").unwrap();
        assert_eq!((art.cols, art.rows), (4, 4));
        assert_eq!(pixels(&art), ["####"; 4]);
    }

    #[test]
    fn short_lines_are_blank_to_the_right() {
        let art = parse("██\n█").unwrap();
        let mut want = vec!["####"; 4];
        want.extend(["##.."; 4]);
        assert_eq!(pixels(&art), want);
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
        assert_eq!((art.cols, art.rows), (160, 38));
    }
}
