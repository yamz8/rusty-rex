//! Cacti and pterodactyls: their sizes, hitboxes and how new ones are picked.

/// Chrome only sends pterodactyls once the game reaches 8.5 px per frame.
pub const PTERODACTYL_MIN_SPEED: f32 = 510.0;
/// Chrome groups small cacti from 4 px per frame, which is below the
/// starting speed, so they can come in groups from the start.
pub const SMALL_GROUP_MIN_SPEED: f32 = 240.0;
/// Chrome only groups large cacti once the game reaches 7 px per frame.
pub const LARGE_GROUP_MIN_SPEED: f32 = 420.0;
/// Height of a pterodactyl's underside for each lane, as in Chrome. The low
/// one has to be jumped, the middle one ducked under and the high one clears
/// a standing rex.
pub const PTERODACTYL_LANES: [f32; 3] = [0.0, 25.0, 50.0];
/// How long each wing position lasts: Chrome flaps at 6 frames a second.
pub const FLAP_TIME: f32 = 1.0 / 6.0;
/// Each pterodactyl flies this much faster or slower than the ground moves,
/// Chrome's 0.8 px/frame.
pub const PTERODACTYL_SPEED_OFFSET: f32 = 48.0;
/// Chrome's gap after an obstacle is its width times the speed in px/frame,
/// plus its kind's minimum gap times this; the largest gap is 1.5 times the
/// smallest.
const GAP_COEFFICIENT: f32 = 0.6;
const MAX_GAP_COEFFICIENT: f32 = 1.5;

const SMALL_CACTUS: (f32, f32) = (17.0, 35.0);
const LARGE_CACTUS: (f32, f32) = (25.0, 50.0);
const PTERODACTYL: (f32, f32) = (46.0, 40.0);

/// A collision box as Chrome lists them: x and y from the top left of the
/// sprite, then width and height.
pub type SpriteBox = (f32, f32, f32, f32);

/// Chrome's collision boxes. They trace each shape roughly, so a near miss
/// past a corner doesn't count as a crash.
const SMALL_CACTUS_BOXES: [SpriteBox; 3] = [
    (0.0, 7.0, 5.0, 27.0),
    (4.0, 0.0, 6.0, 34.0),
    (10.0, 4.0, 7.0, 14.0),
];
const LARGE_CACTUS_BOXES: [SpriteBox; 3] = [
    (0.0, 12.0, 7.0, 38.0),
    (8.0, 0.0, 7.0, 49.0),
    (13.0, 10.0, 10.0, 38.0),
];
const PTERODACTYL_BOXES: [SpriteBox; 5] = [
    (15.0, 15.0, 16.0, 5.0),
    (18.0, 21.0, 24.0, 6.0),
    (2.0, 14.0, 4.0, 3.0),
    (6.0, 10.0, 4.0, 7.0),
    (10.0, 8.0, 6.0, 9.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A group of 1 to 3 small cacti.
    SmallCactus(u8),
    /// A group of 1 to 3 large cacti.
    LargeCactus(u8),
    Pterodactyl,
}

impl Kind {
    /// Whether two kinds count as the same for the no-triple-repeat rule.
    fn same_family(self, other: Kind) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// An axis-aligned box in playfield x and height above the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl Hitbox {
    pub fn new(x: f32, bottom: f32, w: f32, h: f32) -> Self {
        Self {
            left: x,
            bottom,
            right: x + w,
            top: bottom + h,
        }
    }

    /// Places a sprite box for a sprite `h` tall with its bottom-left corner
    /// at (`x`, `bottom`). Chrome's boxes start a pixel in from the sprite's
    /// edge, past its white border.
    pub fn place((bx, by, bw, bh): SpriteBox, x: f32, bottom: f32, h: f32) -> Self {
        let top = bottom + h - 1.0 - by;
        Self {
            left: x + 1.0 + bx,
            right: x + 1.0 + bx + bw,
            top,
            bottom: top - bh,
        }
    }

    /// The rough box Chrome checks before the detailed ones: the sprite
    /// less its one-pixel border.
    pub fn outline(x: f32, bottom: f32, w: f32, h: f32) -> Self {
        Self::new(x + 1.0, bottom + 1.0, w - 2.0, h - 2.0)
    }

    pub fn overlaps(&self, other: &Hitbox) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.bottom < other.top
            && other.bottom < self.top
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Obstacle {
    pub kind: Kind,
    /// Playfield x of the left edge.
    pub x: f32,
    /// Height of the underside above the ground.
    pub bottom: f32,
    /// Seconds since it appeared, for the wing animation.
    pub age: f32,
    /// How much faster than the ground it moves, in px/s.
    pub speed_offset: f32,
    /// Open ground to leave after it before the next obstacle.
    pub gap: f32,
}

impl Obstacle {
    pub fn new(kind: Kind, x: f32, bottom: f32) -> Self {
        Self {
            kind,
            x,
            bottom,
            age: 0.0,
            speed_offset: 0.0,
            gap: 0.0,
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match self.kind {
            Kind::SmallCactus(n) => (SMALL_CACTUS.0 * n as f32, SMALL_CACTUS.1),
            Kind::LargeCactus(n) => (LARGE_CACTUS.0 * n as f32, LARGE_CACTUS.1),
            Kind::Pterodactyl => PTERODACTYL,
        }
    }

    /// The rough box tested first.
    pub fn outline(&self) -> Hitbox {
        let (w, h) = self.size();
        Hitbox::outline(self.x, self.bottom, w, h)
    }

    /// Whether any of the obstacle's collision boxes meets one of `others`.
    pub fn hits(&self, others: &[Hitbox]) -> bool {
        let (w, h) = self.size();
        let (boxes, group): (&[SpriteBox], u8) = match self.kind {
            Kind::SmallCactus(n) => (&SMALL_CACTUS_BOXES, n),
            Kind::LargeCactus(n) => (&LARGE_CACTUS_BOXES, n),
            Kind::Pterodactyl => (&PTERODACTYL_BOXES, 1),
        };
        boxes.iter().enumerate().any(|(i, &b)| {
            let b = stretch(b, i, w, boxes, group);
            let mine = Hitbox::place(b, self.x, self.bottom, h);
            others.iter().any(|o| o.overlaps(&mine))
        })
    }

    /// Chrome's gap for this obstacle at `speed` px/s, picked at random
    /// between its smallest and largest, in whole pixels.
    pub fn choose_gap(&self, speed: f32, rng: &mut Rng) -> f32 {
        let min_gap = match self.kind {
            Kind::SmallCactus(_) | Kind::LargeCactus(_) => 120.0,
            Kind::Pterodactyl => 150.0,
        };
        let smallest = (self.size().0 * speed / 60.0 + min_gap * GAP_COEFFICIENT).round();
        let largest = (smallest * MAX_GAP_COEFFICIENT).round();
        smallest + rng.below((largest - smallest) as u32 + 1) as f32
    }

    pub fn right(&self) -> f32 {
        self.x + self.size().0
    }

    pub fn wings_up(&self) -> bool {
        ((self.age / FLAP_TIME) as u32).is_multiple_of(2)
    }
}

/// Picks the next obstacle for the current speed. `recent` holds the last
/// kinds spawned, newest last, so the same family never comes three times
/// running, which Chrome also forbids.
pub fn choose(rng: &mut Rng, speed: f32, recent: &[Kind], x: f32) -> Obstacle {
    loop {
        let families = if speed >= PTERODACTYL_MIN_SPEED { 3 } else { 2 };
        let (kind, bottom) = match rng.below(families) {
            0 => (
                Kind::SmallCactus(group(rng, speed, SMALL_GROUP_MIN_SPEED)),
                0.0,
            ),
            1 => (
                Kind::LargeCactus(group(rng, speed, LARGE_GROUP_MIN_SPEED)),
                0.0,
            ),
            _ => {
                let lane = rng.below(PTERODACTYL_LANES.len() as u32) as usize;
                (Kind::Pterodactyl, PTERODACTYL_LANES[lane])
            }
        };
        let repeats = recent.len() >= 2
            && recent[recent.len() - 2..]
                .iter()
                .all(|k| k.same_family(kind));
        if !repeats {
            let mut o = Obstacle::new(kind, x, bottom);
            if kind == Kind::Pterodactyl {
                o.speed_offset = if rng.below(2) == 0 {
                    PTERODACTYL_SPEED_OFFSET
                } else {
                    -PTERODACTYL_SPEED_OFFSET
                };
            }
            return o;
        }
    }
}

/// Widens a cactus group's boxes as Chrome does: the middle box stretches
/// across the group and the right one moves to its right end.
fn stretch(b: SpriteBox, i: usize, width: f32, boxes: &[SpriteBox], group: u8) -> SpriteBox {
    if group < 2 {
        return b;
    }
    match i {
        1 => (b.0, b.1, width - boxes[0].2 - boxes[2].2, b.3),
        2 => (width - b.2, b.1, b.2, b.3),
        _ => b,
    }
}

/// A group size of 1 to 3, or always 1 below the speed where groups start.
fn group(rng: &mut Rng, speed: f32, min_speed: f32) -> u8 {
    let max = if speed >= min_speed { 3 } else { 1 };
    1 + rng.below(max) as u8
}

/// Small xorshift generator. Seeded explicitly so tests are repeatable and the
/// game needs no extra crate.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Small seeds have few bits set and xorshift takes a while to spread
        // them, so the seed is scrambled first (splitmix64). Zero is a fixed
        // point of xorshift, so it is avoided too.
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self((z ^ (z >> 31)).max(1))
    }

    /// xorshift64*: the multiply mixes the state's low bits into the high
    /// bits that are returned.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    /// A number in 0..n.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn many(speed: f32) -> Vec<Obstacle> {
        let mut rng = Rng::new(7);
        let mut recent = Vec::new();
        (0..2000)
            .map(|_| {
                let o = choose(&mut rng, speed, &recent, 0.0);
                recent.push(o.kind);
                o
            })
            .collect()
    }

    #[test]
    fn the_start_has_small_groups_but_no_large_ones_or_pterodactyls() {
        let all = many(360.0);
        for o in &all {
            assert!(matches!(
                o.kind,
                Kind::SmallCactus(_) | Kind::LargeCactus(1)
            ));
        }
        assert!(all.iter().any(|o| o.kind == Kind::SmallCactus(3)));
    }

    #[test]
    fn everything_turns_up_at_top_speed() {
        let all = many(780.0);
        for lane in PTERODACTYL_LANES {
            assert!(
                all.iter()
                    .any(|o| o.kind == Kind::Pterodactyl && o.bottom == lane)
            );
        }
        assert!(all.iter().any(|o| o.kind == Kind::SmallCactus(3)));
        assert!(all.iter().any(|o| o.kind == Kind::LargeCactus(3)));
    }

    #[test]
    fn never_three_of_a_family_in_a_row() {
        let all = many(780.0);
        for w in all.windows(3) {
            let same = w[0].kind.same_family(w[1].kind) && w[1].kind.same_family(w[2].kind);
            assert!(!same, "{:?}", w.iter().map(|o| o.kind).collect::<Vec<_>>());
        }
    }

    #[test]
    fn groups_are_wider() {
        let one = Obstacle::new(Kind::LargeCactus(1), 0.0, 0.0);
        let three = Obstacle::new(Kind::LargeCactus(3), 0.0, 0.0);
        assert_eq!(three.size().0, one.size().0 * 3.0);
    }

    #[test]
    fn hitboxes_overlap_only_when_they_meet() {
        let a = Hitbox::new(0.0, 0.0, 10.0, 10.0);
        assert!(a.overlaps(&Hitbox::new(5.0, 5.0, 10.0, 10.0)));
        assert!(!a.overlaps(&Hitbox::new(10.0, 0.0, 10.0, 10.0)));
        assert!(!a.overlaps(&Hitbox::new(0.0, 10.0, 10.0, 10.0)));
    }

    #[test]
    fn sprite_boxes_are_placed_from_the_top_left() {
        // A 2x3 box 1 in and 2 down in a 10 px tall sprite standing on the
        // ground at x 100: Chrome's one-pixel border adds 1 to each.
        let b = Hitbox::place((1.0, 2.0, 2.0, 3.0), 100.0, 0.0, 10.0);
        assert_eq!(b, Hitbox::new(102.0, 4.0, 2.0, 3.0));
    }

    #[test]
    fn group_boxes_stretch_across_the_group() {
        let (w, _) = Obstacle::new(Kind::LargeCactus(3), 0.0, 0.0).size();
        let stretched: Vec<SpriteBox> = (0..3)
            .map(|i| stretch(LARGE_CACTUS_BOXES[i], i, w, &LARGE_CACTUS_BOXES, 3))
            .collect();
        assert_eq!(stretched[0], LARGE_CACTUS_BOXES[0]);
        assert_eq!(stretched[0].2 + stretched[1].2 + stretched[2].2, w);
        assert_eq!(stretched[2].0 + stretched[2].2, w);
        // A single cactus keeps its boxes.
        let single = stretch(LARGE_CACTUS_BOXES[2], 2, 25.0, &LARGE_CACTUS_BOXES, 1);
        assert_eq!(single, LARGE_CACTUS_BOXES[2]);
    }

    #[test]
    fn a_box_grazing_the_corner_of_a_cactus_misses() {
        let cactus = Obstacle::new(Kind::LargeCactus(1), 100.0, 0.0);
        // The top-left corner of the sprite is empty in Chrome's boxes.
        let corner = Hitbox::new(100.0, 45.0, 4.0, 4.0);
        assert!(corner.overlaps(&cactus.outline()));
        assert!(!cactus.hits(&[corner]));
        let trunk = Hitbox::new(112.0, 20.0, 2.0, 2.0);
        assert!(cactus.hits(&[trunk]));
    }

    #[test]
    fn gaps_follow_chromes_formula() {
        let mut rng = Rng::new(3);
        let small = Obstacle::new(Kind::SmallCactus(1), 0.0, 0.0);
        // At 6 px/frame: 17 * 6 + 120 * 0.6 = 174, up to 261.
        for _ in 0..200 {
            let gap = small.choose_gap(360.0, &mut rng);
            assert!((174.0..=261.0).contains(&gap), "{gap}");
            assert_eq!(gap, gap.round());
        }
        let ptero = Obstacle::new(Kind::Pterodactyl, 0.0, 0.0);
        let gaps: Vec<f32> = (0..200)
            .map(|_| ptero.choose_gap(780.0, &mut rng))
            .collect();
        // 46 * 13 + 150 * 0.6 = 688, up to 1032.
        assert!(gaps.iter().all(|g| (688.0..=1032.0).contains(g)));
        assert!(gaps.iter().any(|&g| g < 720.0) && gaps.iter().any(|&g| g > 1000.0));
    }

    #[test]
    fn pterodactyls_fly_a_little_faster_or_slower() {
        let all = many(780.0);
        let offsets: Vec<f32> = all
            .iter()
            .filter(|o| o.kind == Kind::Pterodactyl)
            .map(|o| o.speed_offset)
            .collect();
        assert!(offsets.contains(&PTERODACTYL_SPEED_OFFSET));
        assert!(offsets.contains(&-PTERODACTYL_SPEED_OFFSET));
        assert!(
            all.iter()
                .filter(|o| o.kind != Kind::Pterodactyl)
                .all(|o| o.speed_offset == 0.0)
        );
    }

    #[test]
    fn wings_flap() {
        let mut o = Obstacle::new(Kind::Pterodactyl, 0.0, 0.0);
        let start = o.wings_up();
        o.age = FLAP_TIME;
        assert_ne!(o.wings_up(), start);
    }

    #[test]
    fn rng_is_repeatable() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        for seed in 0..10 {
            assert_ne!(Rng::new(seed).next_u32(), 0, "seed {seed}");
        }
        assert_ne!(Rng::new(1).next_u32(), Rng::new(2).next_u32());
    }
}
