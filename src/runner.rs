//! Who runs: Chrome's rex, the Omarchy runner (the Omarchy logo in Omi's
//! shapes on legs) or block or braille art from a file. The physics are the
//! same for all; each has its own size and collision boxes.

use crate::obstacle::SpriteBox;
use crate::rex::{DUCKING_SIZE, Pose, STANDING_SIZE};
use crate::sprites::Sprite;

/// Collision boxes are given in a frame this tall, the standing rex's, with
/// a shorter body at its bottom, as Chrome gives them.
pub const FRAME_HEIGHT: f32 = 47.0;

/// Chrome's collision boxes for the rex, standing and ducking.
const REX_RUNNING_BOXES: [SpriteBox; 6] = [
    (22.0, 0.0, 17.0, 16.0),
    (1.0, 18.0, 30.0, 9.0),
    (10.0, 35.0, 14.0, 8.0),
    (1.0, 24.0, 29.0, 5.0),
    (5.0, 30.0, 21.0, 4.0),
    (9.0, 34.0, 15.0, 4.0),
];
const REX_DUCKING_BOXES: [SpriteBox; 1] = [(1.0, 18.0, 55.0, 25.0)];

/// The Omarchy runner's grids are 15 cells wide, 18 tall standing and 9
/// ducking. Standing, it is the rex's 47 px tall, so a cell is 47/18 px
/// and it is as wide as Omi's ratio makes it, 39 px.
pub const OMARCHY_CELL: f32 = FRAME_HEIGHT / 18.0;
pub const OMARCHY_STANDING_SIZE: (f32, f32) = (15.0 * OMARCHY_CELL, 18.0 * OMARCHY_CELL);
pub const OMARCHY_DUCKING_SIZE: (f32, f32) = (15.0 * OMARCHY_CELL, 9.0 * OMARCHY_CELL);
/// Its boxes trace the sprite, in the FRAME_HEIGHT frame: standing, the
/// logo's square and the legs under it; ducking, the folded square and its
/// feet. Like Chrome's, they stop short of the outermost pixel.
const OMARCHY_RUNNING_BOXES: [SpriteBox; 2] = [(1.0, 1.0, 37.0, 38.0), (7.0, 39.0, 25.0, 7.0)];
const OMARCHY_DUCKING_BOXES: [SpriteBox; 2] = [(1.0, 24.0, 37.0, 20.0), (8.0, 44.0, 20.0, 2.0)];

/// The most pixels art may have each way, 48 characters by 24 lines.
/// Finer art would only shrink past what shows, and every run of pixels is
/// a rectangle to draw.
const ART_MAX_COLS: usize = 96;
const ART_MAX_ROWS: usize = 96;

/// A runner drawn from block or braille art. Its square pixels are as large
/// as fit the rex's standing box, so it is never harder to get past
/// obstacles than the rex; ducking folds it to half its height.
#[derive(Debug)]
pub struct Art {
    pub standing: Sprite,
    pub ducking: Sprite,
    standing_size: (f32, f32),
    ducking_size: (f32, f32),
    running_boxes: Vec<SpriteBox>,
    ducking_boxes: Vec<SpriteBox>,
}

impl Art {
    /// Builds a runner from lit pixels, row by row, all rows as wide.
    pub fn new(grid: &[Vec<bool>]) -> Result<Self, String> {
        let (rows, cols) = (grid.len(), grid.first().map_or(0, Vec::len));
        if !grid.iter().flatten().any(|&p| p) {
            return Err("has no block or braille art to draw".to_owned());
        }
        if cols > ART_MAX_COLS || rows > ART_MAX_ROWS {
            return Err(format!(
                "is too big: a runner is at most {} characters wide and {} lines tall",
                ART_MAX_COLS / 2,
                ART_MAX_ROWS / 4
            ));
        }
        let cell = (FRAME_HEIGHT / rows as f32).min(STANDING_SIZE.0 / cols as f32);
        let standing_size = (cols as f32 * cell, rows as f32 * cell);
        let ducking_size = (standing_size.0, standing_size.1 / 2.0);
        // Two rows fold into one, lit where either is.
        let ducked: Vec<Vec<bool>> = grid
            .chunks(2)
            .map(|pair| (0..cols).map(|x| pair.iter().any(|row| row[x])).collect())
            .collect();
        Ok(Self {
            running_boxes: trace(grid, standing_size),
            ducking_boxes: trace(&ducked, ducking_size),
            standing: Sprite::from_pixels(grid),
            ducking: Sprite::from_pixels(&ducked),
            standing_size,
            ducking_size,
        })
    }
}

/// Every runner from art is a different one, and Runner compares them only
/// to tell runners apart.
impl PartialEq for Art {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Art {}

/// Collision boxes for art drawn `size` big at the frame's bottom. The rows
/// are cut into as many bands as a runner may have boxes, and each band is
/// boxed around what it lights, so the boxes follow the outline from top to
/// bottom. Like Chrome's, they stop short of the outermost pixel.
fn trace(grid: &[Vec<bool>], (w, h): (f32, f32)) -> Vec<SpriteBox> {
    let (rows, cols) = (grid.len(), grid[0].len());
    let (sx, sy) = (w / cols as f32, h / rows as f32);
    let bands = rows.min(Runner::MAX_BOXES);
    let mut boxes = Vec::new();
    for band in 0..bands {
        let lit: Vec<(usize, usize, usize)> = (band * rows / bands..(band + 1) * rows / bands)
            .filter_map(|y| {
                let left = grid[y].iter().position(|&p| p)?;
                let right = grid[y].iter().rposition(|&p| p)?;
                Some((y, left, right))
            })
            .collect();
        let (Some(first), Some(last)) = (lit.first(), lit.last()) else {
            continue;
        };
        let left = lit.iter().map(|l| l.1).min().unwrap_or(0) as f32 * sx;
        let right = (lit.iter().map(|l| l.2).max().unwrap_or(0) + 1) as f32 * sx;
        let top = FRAME_HEIGHT - h + first.0 as f32 * sy;
        let bottom = FRAME_HEIGHT - h + (last.0 + 1) as f32 * sy;
        let (bw, bh) = (right - left, bottom - top);
        // A pixel in, but never so far that a thin part stops counting.
        let (ix, iy) = ((bw / 4.0).min(1.0), (bh / 4.0).min(1.0));
        boxes.push((left + ix, top + iy, bw - 2.0 * ix, bh - 2.0 * iy));
    }
    boxes
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Runner {
    #[default]
    Rex,
    Omarchy,
    /// Made once at startup and kept for the whole run, so the game stays
    /// cheap to copy for the autopilot.
    Art(&'static Art),
}

impl Runner {
    /// Width and height drawn for a pose.
    pub fn size(self, pose: Pose) -> (f32, f32) {
        match (self, pose == Pose::Ducking) {
            (Runner::Rex, false) => STANDING_SIZE,
            (Runner::Rex, true) => DUCKING_SIZE,
            (Runner::Omarchy, false) => OMARCHY_STANDING_SIZE,
            (Runner::Omarchy, true) => OMARCHY_DUCKING_SIZE,
            (Runner::Art(art), false) => art.standing_size,
            (Runner::Art(art), true) => art.ducking_size,
        }
    }

    /// The standing size, which Chrome also uses for its rough first test.
    pub fn standing(self) -> (f32, f32) {
        self.size(Pose::Running)
    }

    /// The detailed boxes for a pose, in a FRAME_HEIGHT frame.
    pub fn boxes(self, pose: Pose) -> &'static [SpriteBox] {
        match (self, pose == Pose::Ducking) {
            (Runner::Rex, false) => &REX_RUNNING_BOXES,
            (Runner::Rex, true) => &REX_DUCKING_BOXES,
            (Runner::Omarchy, false) => &OMARCHY_RUNNING_BOXES,
            (Runner::Omarchy, true) => &OMARCHY_DUCKING_BOXES,
            (Runner::Art(art), false) => &art.running_boxes,
            (Runner::Art(art), true) => &art.ducking_boxes,
        }
    }

    /// The most boxes any pose has, for a fixed-size buffer.
    pub const MAX_BOXES: usize = REX_RUNNING_BOXES.len();
}

/// Runners from art of every awkward shape, for tests: a solid square as
/// big as fits, a wide flat bar, a thin pole, a single pixel, a hollow
/// ring, and braille as fine as allowed.
#[cfg(test)]
pub fn test_arts() -> Vec<Runner> {
    let grid = |cols: usize, rows: usize, lit: &dyn Fn(usize, usize) -> bool| -> Vec<Vec<bool>> {
        (0..rows)
            .map(|y| (0..cols).map(|x| lit(x, y)).collect())
            .collect()
    };
    let ring = |x: usize, y: usize| x < 3 || y < 3 || x > 20 || y > 20;
    let checks = |x: usize, y: usize| (x + y).is_multiple_of(2);
    [
        grid(44, 47, &|_, _| true),
        grid(80, 8, &|_, _| true),
        grid(2, 40, &|_, _| true),
        grid(1, 1, &|_, _| true),
        grid(24, 24, &ring),
        grid(ART_MAX_COLS, ART_MAX_ROWS, &checks),
    ]
    .iter()
    .map(|g| Runner::Art(Box::leak(Box::new(Art::new(g).unwrap()))))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::PTERODACTYL_LANES;

    /// The highest point of a pose's boxes, above the ground.
    fn top(r: Runner, pose: Pose) -> f32 {
        r.boxes(pose)
            .iter()
            .map(|b| FRAME_HEIGHT - 1.0 - b.1)
            .fold(0.0, f32::max)
    }

    #[test]
    fn runners_from_art_duck_under_the_middle_lane_and_pass_under_the_high_one() {
        // Art may be too short to stand into the middle lane; then it
        // simply runs under it.
        let middle = PTERODACTYL_LANES[1] + 13.0;
        let high = PTERODACTYL_LANES[2] + 13.0;
        for r in test_arts() {
            assert!(top(r, Pose::Ducking) < middle, "{r:?} ducks too tall");
            assert!(top(r, Pose::Running) < high, "{r:?} hits the high lane");
        }
    }

    #[test]
    fn art_fits_the_rex_box_with_square_pixels() {
        for r in test_arts() {
            let Runner::Art(art) = r else { unreachable!() };
            let (w, h) = r.standing();
            assert!(
                w <= STANDING_SIZE.0 + 1e-3 && h <= FRAME_HEIGHT + 1e-3,
                "{w}x{h}"
            );
            assert!(
                w >= STANDING_SIZE.0 - 1e-3 || h >= FRAME_HEIGHT - 1e-3,
                "{w}x{h}"
            );
            let (px, py) = (w / art.standing.cols as f32, h / art.standing.rows as f32);
            assert!((px - py).abs() < 1e-3, "{px} by {py}");
            assert_eq!(r.size(Pose::Ducking), (w, h / 2.0));
        }
    }

    #[test]
    fn boxes_trace_the_art() {
        let boxes = |rows: &[&str]| {
            let grid: Vec<Vec<bool>> = rows
                .iter()
                .map(|r| r.chars().map(|c| c == '#').collect())
                .collect();
            let art = Art::new(&grid).unwrap();
            art.running_boxes
        };
        // A solid block is one box per band, together the whole block a
        // pixel in: 47 rows, so a cell is a pixel.
        let solid = boxes(&["#"; 47]);
        assert_eq!(solid.len(), Runner::MAX_BOXES);
        assert_eq!(solid[0].1, 1.0);
        for b in &solid {
            assert_eq!((b.0, b.2), (0.25, 0.5));
        }
        // A T: the top band is the bar, the lower ones only the stem.
        let mut t = vec!["#######"];
        t.extend(["...#..."; 5]);
        let t = boxes(&t);
        assert_eq!(t.len(), Runner::MAX_BOXES);
        assert!(t[0].2 > 5.0 * t[1].2, "{t:?}");
        // Blank rows in between leave no box.
        assert_eq!(boxes(&["#", ".", ".", ".", ".", "#"]).len(), 2);
    }

    #[test]
    fn art_with_nothing_to_draw_or_too_big_is_refused() {
        assert!(Art::new(&[]).is_err());
        assert!(Art::new(&[vec![false; 3]]).is_err());
        assert!(Art::new(&vec![vec![true; ART_MAX_COLS]; ART_MAX_ROWS]).is_ok());
        let wide = Art::new(&[vec![true; ART_MAX_COLS + 1]]).unwrap_err();
        assert!(
            wide.contains("48 characters wide and 24 lines tall"),
            "{wide}"
        );
        assert!(Art::new(&vec![vec![true]; ART_MAX_ROWS + 1]).is_err());
    }

    #[test]
    fn every_runner_ducks_under_the_middle_lane_and_stands_into_it() {
        // A pterodactyl's lowest box is 13 px above its underside.
        let middle = PTERODACTYL_LANES[1] + 13.0;
        let high = PTERODACTYL_LANES[2] + 13.0;
        for r in [Runner::Rex, Runner::Omarchy] {
            assert!(top(r, Pose::Ducking) < middle, "{r:?} ducks too tall");
            assert!(top(r, Pose::Running) > middle, "{r:?} stands too short");
            assert!(top(r, Pose::Running) < high, "{r:?} hits the high lane");
        }
    }

    #[test]
    fn boxes_fit_their_sprites() {
        for runner in [Runner::Rex, Runner::Omarchy]
            .into_iter()
            .chain(test_arts())
        {
            for pose in [Pose::Running, Pose::Ducking] {
                let (w, h) = runner.size(pose);
                for &(x, y, bw, bh) in runner.boxes(pose) {
                    assert!(x >= 0.0 && x + bw <= w, "{runner:?} {pose:?}");
                    // inside the sprite, which stands at the frame's bottom
                    assert!(y >= FRAME_HEIGHT - h - 1.0, "{runner:?} {pose:?}");
                    assert!(y + bh <= FRAME_HEIGHT, "{runner:?} {pose:?}");
                }
                assert!(runner.boxes(pose).len() <= Runner::MAX_BOXES);
            }
        }
    }
}
