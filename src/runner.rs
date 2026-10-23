//! Who runs: Chrome's rex, or the Omarchy runner, the Omarchy logo in Omi's
//! shapes on legs. The physics are the same for both; each has its own size
//! and collision boxes.

use crate::obstacle::SpriteBox;
use crate::rex::{DUCKING_SIZE, Pose, STANDING_SIZE};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Runner {
    #[default]
    Rex,
    Omarchy,
}

impl Runner {
    /// Width and height drawn for a pose.
    pub fn size(self, pose: Pose) -> (f32, f32) {
        match (self, pose == Pose::Ducking) {
            (Runner::Rex, false) => STANDING_SIZE,
            (Runner::Rex, true) => DUCKING_SIZE,
            (Runner::Omarchy, false) => OMARCHY_STANDING_SIZE,
            (Runner::Omarchy, true) => OMARCHY_DUCKING_SIZE,
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
        }
    }

    /// The most boxes any pose has, for a fixed-size buffer.
    pub const MAX_BOXES: usize = REX_RUNNING_BOXES.len();
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
        for runner in [Runner::Rex, Runner::Omarchy] {
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
