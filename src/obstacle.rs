//! Cacti and pterodactyls: their sizes, hitboxes and how new ones are picked.

/// Chrome only sends pterodactyls once the game reaches 8.5 px per frame.
pub const PTERODACTYL_MIN_SPEED: f32 = 510.0;
/// Chrome only groups cacti once the game reaches 7 px per frame.
pub const GROUP_MIN_SPEED: f32 = 420.0;
/// Height of a pterodactyl's underside for each lane. The low one has to be
/// jumped, the middle one ducked under (a ducking rex is 30 tall) and the high
/// one clears a standing rex (47 tall).
pub const PTERODACTYL_LANES: [f32; 3] = [0.0, 35.0, 55.0];
/// How long each wing position lasts.
pub const FLAP_TIME: f32 = 0.15;

const SMALL_CACTUS: (f32, f32) = (17.0, 35.0);
const LARGE_CACTUS: (f32, f32) = (25.0, 50.0);
const PTERODACTYL: (f32, f32) = (46.0, 40.0);
/// Hitboxes are shrunk by this much on every side, because the drawn shapes
/// don't fill their boxes and a pixel-perfect box feels unfair.
const HITBOX_INSET: f32 = 3.0;

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

    pub fn inset(self, by: f32) -> Self {
        Self {
            left: self.left + by,
            bottom: self.bottom + by,
            right: self.right - by,
            top: self.top - by,
        }
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
}

impl Obstacle {
    pub fn new(kind: Kind, x: f32, bottom: f32) -> Self {
        Self {
            kind,
            x,
            bottom,
            age: 0.0,
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match self.kind {
            Kind::SmallCactus(n) => (SMALL_CACTUS.0 * n as f32, SMALL_CACTUS.1),
            Kind::LargeCactus(n) => (LARGE_CACTUS.0 * n as f32, LARGE_CACTUS.1),
            Kind::Pterodactyl => PTERODACTYL,
        }
    }

    pub fn hitbox(&self) -> Hitbox {
        let (w, h) = self.size();
        Hitbox::new(self.x, self.bottom, w, h).inset(HITBOX_INSET)
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
        let max_group = if speed >= GROUP_MIN_SPEED { 3 } else { 1 };
        let group = 1 + rng.below(max_group) as u8;
        let (kind, bottom) = match rng.below(families) {
            0 => (Kind::SmallCactus(group), 0.0),
            1 => (Kind::LargeCactus(group), 0.0),
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
            return Obstacle::new(kind, x, bottom);
        }
    }
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

    /// A number in 0.0..1.0.
    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
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
    fn no_pterodactyls_or_groups_at_the_start() {
        for o in many(360.0) {
            assert!(matches!(
                o.kind,
                Kind::SmallCactus(1) | Kind::LargeCactus(1)
            ));
        }
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
