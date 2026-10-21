//! Pixel art for the rex and obstacles, as text grids.
//!
//! `#` is drawn in the sprite's colour, `o` in the background colour (eyes)
//! and `.` is left empty. Each sprite is stretched over its hitbox, so the
//! grids only need roughly the right proportions. Keeping the art as data
//! means its shape can be tested without a window.

pub type Grid = &'static [&'static str];

const REX_HEAD: Grid = &[
    "............########..",
    "...........##########.",
    "...........##o########",
    "...........###########",
    "...........###########",
    "...........#####......",
    "...........#########..",
];

/// Chrome's crashed rex stares wide-eyed.
const REX_HEAD_CRASHED: Grid = &[
    "............########..",
    "...........#oo########",
    "...........#oo########",
    "...........###########",
    "...........###########",
    "...........#####......",
    "...........#########..",
];

const REX_BODY: Grid = &[
    "#.........#####.......",
    "#........######.......",
    "##......########......",
    "###....##########.....",
    "###..#########..#.....",
    "##############..#.....",
    "##############........",
    ".############.........",
    "..###########.........",
    "...#########..........",
    "....#######...........",
    ".....######...........",
];

const REX_LEGS_DOWN: Grid = &[
    ".....###.##...........",
    ".....##...#...........",
    ".....#....#...........",
    ".....##...##..........",
];

const REX_LEGS_BACK_UP: Grid = &[
    ".....###.##...........",
    "....###...#...........",
    "..........#...........",
    "..........##..........",
];

const REX_LEGS_FRONT_UP: Grid = &[
    ".....###.##...........",
    ".....##...###.........",
    ".....#................",
    ".....##...............",
];

const DUCK_BODY: Grid = &[
    "....................########..",
    "...................##o#######.",
    "##.......#####################",
    "#############################.",
    ".########################.....",
    "..#####################.......",
    "...##################.##......",
    "....###############...........",
    ".....############.............",
    "......#########...............",
];

const DUCK_BODY_CRASHED: Grid = &[
    "....................########..",
    "...................#oo#######.",
    "##.................#oo########",
    "#############################.",
    ".########################.....",
    "..#####################.......",
    "...##################.##......",
    "....###############...........",
    ".....############.............",
    "......#########...............",
];

const DUCK_LEGS_DOWN: Grid = &[
    "......###.##..................",
    "......##...#..................",
    "......#....#..................",
    "......##...##.................",
];

const DUCK_LEGS_BACK_UP: Grid = &[
    "......###.##..................",
    ".....###...#..................",
    "...........#..................",
    "...........##.................",
];

const DUCK_LEGS_FRONT_UP: Grid = &[
    "......###.##..................",
    "......##...###................",
    "......#.......................",
    "......##......................",
];

pub const SMALL_CACTUS: Grid = &[
    "...##....",
    "..####...",
    "..####..#",
    "#.####..#",
    "#.####.##",
    "#.######.",
    "########.",
    ".#####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
    "..####...",
];

pub const LARGE_CACTUS: Grid = &[
    ".....###.....",
    "....#####....",
    "....#####..##",
    "##..#####..##",
    "##..#####..##",
    "##..#####..##",
    "##..#####..##",
    "##..#########",
    "##..########.",
    "###.#####....",
    ".########....",
    "..#######....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
    "....#####....",
];

/// Faces left, the way it flies.
pub const PTERODACTYL_WINGS_UP: Grid = &[
    "..........#............",
    "..........##...........",
    "..........###..........",
    "..........####.........",
    "..........#####........",
    "....##....######.......",
    "...####...#######......",
    "..##o###..########.....",
    "#########.#########....",
    "...####################",
    "......############.....",
    "........#########......",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
];

pub const PTERODACTYL_WINGS_DOWN: Grid = &[
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    ".......................",
    "....##.................",
    "...####................",
    "..##o###...............",
    "#########..............",
    "...####################",
    "......############.....",
    "........##########.....",
    "..........#######......",
    "..........######.......",
    "..........#####........",
    "..........####.........",
    "..........###..........",
    "..........##...........",
    "..........#............",
    ".......................",
];

/// Which way the rex's legs are drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legs {
    Down,
    BackUp,
    FrontUp,
}

/// Builds the rex's rows from head, body and legs, so the variants don't each
/// repeat the whole drawing.
pub fn rex(ducking: bool, crashed: bool, legs: Legs) -> Vec<&'static str> {
    let mut rows = Vec::new();
    if ducking {
        rows.extend_from_slice(if crashed {
            DUCK_BODY_CRASHED
        } else {
            DUCK_BODY
        });
        rows.extend_from_slice(match legs {
            Legs::Down => DUCK_LEGS_DOWN,
            Legs::BackUp => DUCK_LEGS_BACK_UP,
            Legs::FrontUp => DUCK_LEGS_FRONT_UP,
        });
    } else {
        rows.extend_from_slice(if crashed { REX_HEAD_CRASHED } else { REX_HEAD });
        rows.extend_from_slice(REX_BODY);
        rows.extend_from_slice(match legs {
            Legs::Down => REX_LEGS_DOWN,
            Legs::BackUp => REX_LEGS_BACK_UP,
            Legs::FrontUp => REX_LEGS_FRONT_UP,
        });
    }
    rows
}

/// A horizontal stretch of same-coloured pixels. Drawing runs instead of
/// single pixels means far fewer rectangles and no seams inside a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub x: usize,
    pub y: usize,
    pub len: usize,
    /// Drawn in the background colour rather than the sprite's.
    pub eye: bool,
}

pub fn runs(rows: &[&str]) -> Vec<Run> {
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let cells: Vec<char> = row.chars().collect();
        let mut x = 0;
        while x < cells.len() {
            let c = cells[x];
            let start = x;
            while x < cells.len() && cells[x] == c {
                x += 1;
            }
            if c != '.' {
                out.push(Run {
                    x: start,
                    y,
                    len: x - start,
                    eye: c == 'o',
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_rex() -> Vec<Vec<&'static str>> {
        let mut all = Vec::new();
        for ducking in [false, true] {
            for crashed in [false, true] {
                for legs in [Legs::Down, Legs::BackUp, Legs::FrontUp] {
                    all.push(rex(ducking, crashed, legs));
                }
            }
        }
        all
    }

    fn assert_rectangular(rows: &[&str]) {
        let width = rows[0].len();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), width, "row {i} of {rows:#?}");
            assert!(row.chars().all(|c| matches!(c, '#' | 'o' | '.')), "{row}");
        }
    }

    #[test]
    fn every_sprite_is_a_rectangle() {
        for rows in all_rex() {
            assert_rectangular(&rows);
        }
        for grid in [
            SMALL_CACTUS,
            LARGE_CACTUS,
            PTERODACTYL_WINGS_UP,
            PTERODACTYL_WINGS_DOWN,
        ] {
            assert_rectangular(grid);
        }
    }

    #[test]
    fn rex_variants_share_a_size() {
        let standing = rex(false, false, Legs::Down);
        let ducking = rex(true, false, Legs::Down);
        for rows in all_rex() {
            let expected = if rows[0].len() == ducking[0].len() {
                &ducking
            } else {
                &standing
            };
            assert_eq!(rows.len(), expected.len());
        }
    }

    /// Stretching a grid over its hitbox should not distort it much.
    #[test]
    fn grids_match_their_hitbox_proportions() {
        use crate::obstacle::{Kind, Obstacle};
        use crate::rex::{DUCKING_SIZE, STANDING_SIZE};

        let check = |rows: &[&str], (w, h): (f32, f32)| {
            let sx = w / rows[0].len() as f32;
            let sy = h / rows.len() as f32;
            assert!((sx / sy - 1.0).abs() < 0.2, "{sx} x {sy} for {w}x{h}");
        };
        check(&rex(false, false, Legs::Down), STANDING_SIZE);
        check(&rex(true, false, Legs::Down), DUCKING_SIZE);
        check(
            SMALL_CACTUS,
            Obstacle::new(Kind::SmallCactus(1), 0.0, 0.0).size(),
        );
        check(
            LARGE_CACTUS,
            Obstacle::new(Kind::LargeCactus(1), 0.0, 0.0).size(),
        );
        check(
            PTERODACTYL_WINGS_UP,
            Obstacle::new(Kind::Pterodactyl, 0.0, 0.0).size(),
        );
    }

    #[test]
    fn standing_feet_touch_the_bottom_row() {
        for legs in [Legs::Down, Legs::BackUp, Legs::FrontUp] {
            let rows = rex(false, false, legs);
            assert!(rows.last().unwrap().contains('#'));
        }
    }

    #[test]
    fn running_frames_differ_and_crashing_changes_the_eye() {
        assert_ne!(
            rex(false, false, Legs::BackUp),
            rex(false, false, Legs::FrontUp)
        );
        let normal = runs(&rex(false, false, Legs::Down));
        let crashed = runs(&rex(false, true, Legs::Down));
        let eye = |r: &[Run]| r.iter().filter(|r| r.eye).map(|r| r.len).sum::<usize>();
        assert!(eye(&crashed) > eye(&normal));
    }

    #[test]
    fn runs_cover_exactly_the_drawn_cells() {
        let rows = ["#.##o", "..ooo"];
        assert_eq!(
            runs(&rows),
            vec![
                Run {
                    x: 0,
                    y: 0,
                    len: 1,
                    eye: false
                },
                Run {
                    x: 2,
                    y: 0,
                    len: 2,
                    eye: false
                },
                Run {
                    x: 4,
                    y: 0,
                    len: 1,
                    eye: true
                },
                Run {
                    x: 2,
                    y: 1,
                    len: 3,
                    eye: true
                },
            ]
        );
    }
}
