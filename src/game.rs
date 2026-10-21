//! The running game: the world scrolling past the rex, getting faster, with
//! obstacles to clear.
//!
//! Like the rex, this never touches macroquad. Horizontal positions are in
//! playfield pixels and heights in pixels above the ground.

use crate::obstacle::{self, Hitbox, Kind, Obstacle, Rng};
use crate::rex::{Input, Rex};

/// Where the rex stands; the world scrolls past it instead of it moving.
pub const REX_X: f32 = 60.0;
/// Chrome starts at 6 px per frame at 60 fps.
pub const START_SPEED: f32 = 360.0;
/// Chrome tops out at 13 px per frame at 60 fps.
pub const MAX_SPEED: f32 = 780.0;
/// Chrome adds 0.001 px per frame every frame, which is 3.6 px/s each second,
/// so it takes about two minutes to reach top speed.
pub const ACCELERATION: f32 = 3.6;
/// New obstacles appear here, past the right edge of the playfield, so they
/// slide in rather than pop up even when the window shows a little extra.
pub const SPAWN_X: f32 = 1200.0;
/// Seconds of open ground between one obstacle leaving and the next arriving
/// at the slowest; a full jump lasts 0.65 s, so there is always room to land
/// and jump again.
const MIN_GAP_TIME: f32 = 0.8;
/// Gaps vary up to this multiple of the minimum.
const MAX_GAP_FACTOR: f32 = 1.5;
/// Distance before the first obstacle, giving the player a moment to settle.
const FIRST_SPAWN_DISTANCE: f32 = 360.0;
/// Obstacles this far left of the playfield are gone for good.
const DESPAWN_X: f32 = -300.0;
/// After a crash, jump is ignored this long so a panicked press doesn't skip
/// straight past the game-over screen.
pub const RESTART_DELAY: f32 = 0.5;
/// Chrome's score is the distance in pixels times this.
const SCORE_PER_PIXEL: f32 = 0.025;
/// Every this many points the score blinks, as in Chrome.
pub const MILESTONE: u32 = 100;
/// How long the score blinks after a milestone.
pub const MILESTONE_FLASH: f32 = 0.8;
/// The rex's hitbox is trimmed more than the obstacles', since its blocky
/// stand-in art has empty corners around the head and tail.
const REX_HITBOX_INSET: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Before the first jump, as in Chrome.
    Waiting,
    Running,
    /// Crashed; holds the seconds since the crash.
    Over(f32),
}

#[derive(Debug)]
pub struct Game {
    pub state: State,
    pub rex: Rex,
    /// Scroll speed in pixels per second.
    pub speed: f32,
    /// How far the world has scrolled in this run.
    pub distance: f32,
    pub obstacles: Vec<Obstacle>,
    /// Best score so far, including earlier sessions once loaded from disk.
    pub high_score: u32,
    /// Seconds of score blinking left after the last milestone.
    pub milestone_flash: f32,
    rng: Rng,
    /// Distance left before the next obstacle spawns.
    until_spawn: f32,
    /// The last two kinds spawned, newest last.
    recent: Vec<Kind>,
    jump_was_down: bool,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            state: State::Waiting,
            rex: Rex::new(),
            speed: START_SPEED,
            distance: 0.0,
            obstacles: Vec::new(),
            high_score: 0,
            milestone_flash: 0.0,
            rng: Rng::new(seed),
            until_spawn: FIRST_SPAWN_DISTANCE,
            recent: Vec::new(),
            jump_was_down: false,
        }
    }

    pub fn update(&mut self, dt: f32, input: Input) {
        let jump_pressed = input.jump && !self.jump_was_down;
        self.jump_was_down = input.jump;

        match self.state {
            State::Waiting => {
                if jump_pressed {
                    self.state = State::Running;
                    self.run(dt, input);
                }
            }
            State::Running => self.run(dt, input),
            State::Over(since) => {
                let since = since + dt;
                self.state = State::Over(since);
                if jump_pressed && since >= RESTART_DELAY {
                    self.restart();
                }
            }
        }
    }

    pub fn score(&self) -> u32 {
        (self.distance * SCORE_PER_PIXEL) as u32
    }

    pub fn rex_hitbox(&self) -> Hitbox {
        let (w, h) = self.rex.size();
        Hitbox::new(REX_X, self.rex.height, w, h).inset(REX_HITBOX_INSET)
    }

    fn run(&mut self, dt: f32, input: Input) {
        self.rex.update(dt, input);

        let step = self.speed * dt;
        let before = self.score();
        self.distance += step;
        self.milestone_flash = (self.milestone_flash - dt).max(0.0);
        if self.score() / MILESTONE > before / MILESTONE {
            self.milestone_flash = MILESTONE_FLASH;
        }
        self.speed = (self.speed + ACCELERATION * dt).min(MAX_SPEED);

        for o in &mut self.obstacles {
            o.x -= step;
            o.age += dt;
        }
        self.obstacles.retain(|o| o.right() > DESPAWN_X);

        self.until_spawn -= step;
        if self.until_spawn <= 0.0 {
            self.spawn();
        }

        let rex = self.rex_hitbox();
        if self.obstacles.iter().any(|o| o.hitbox().overlaps(&rex)) {
            self.state = State::Over(0.0);
            self.milestone_flash = 0.0;
            self.high_score = self.high_score.max(self.score());
        }
    }

    fn spawn(&mut self) {
        let o = obstacle::choose(&mut self.rng, self.speed, &self.recent, SPAWN_X);
        let gap = self.speed * MIN_GAP_TIME * (1.0 + (MAX_GAP_FACTOR - 1.0) * self.rng.unit());
        self.until_spawn = o.size().0 + gap;
        self.recent.push(o.kind);
        if self.recent.len() > 2 {
            self.recent.remove(0);
        }
        self.obstacles.push(o);
    }

    fn restart(&mut self) {
        let rng = self.rng.clone();
        *self = Self {
            state: State::Running,
            rng,
            high_score: self.high_score,
            ..Self::new(0)
        };
        self.jump_was_down = true;
        self.rex.ignore_held_jump();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::PTERODACTYL_LANES;
    use crate::rex::Pose;

    const DT: f32 = 1.0 / 120.0;
    const NONE: Input = Input {
        jump: false,
        duck: false,
    };
    const JUMP: Input = Input {
        jump: true,
        duck: false,
    };
    const DUCK: Input = Input {
        jump: false,
        duck: true,
    };

    fn run(game: &mut Game, seconds: f32, input: Input) {
        for _ in 0..(seconds / DT).round() as u32 {
            game.update(DT, input);
        }
    }

    /// A running game with no random spawns, holding just `obstacle`.
    fn course(obstacle: Obstacle) -> Game {
        let mut game = Game::new(1);
        game.update(DT, JUMP);
        game.update(DT, NONE);
        game.rex = Rex::new();
        game.until_spawn = f32::INFINITY;
        game.obstacles = vec![obstacle];
        game
    }

    /// Plays until the obstacle is behind the rex or the rex crashes, calling
    /// `input` with the obstacle's distance ahead of the rex each tick.
    fn play(game: &mut Game, input: impl Fn(f32) -> Input) {
        while game.state == State::Running && game.obstacles[0].right() > REX_X - 10.0 {
            let ahead = game.obstacles[0].x - (REX_X + game.rex.size().0);
            game.update(DT, input(ahead));
        }
    }

    #[test]
    fn waits_for_the_first_jump() {
        let mut game = Game::new(1);
        run(&mut game, 2.0, NONE);
        assert_eq!(game.state, State::Waiting);
        assert_eq!(game.distance, 0.0);

        game.update(DT, JUMP);
        assert_eq!(game.state, State::Running);
        assert_eq!(game.rex.pose, Pose::Jumping);
    }

    #[test]
    fn starts_at_chrome_speed() {
        let game = Game::new(1);
        assert_eq!(game.speed, START_SPEED);
    }

    #[test]
    fn scrolls_by_speed_times_time() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e6, 0.0));
        let start = game.distance;
        run(&mut game, 1.0, NONE);
        // The speed creeps up during the second, so allow a little over.
        let moved = game.distance - start;
        assert!((START_SPEED..START_SPEED + ACCELERATION).contains(&moved));
    }

    #[test]
    fn speeds_up_and_is_capped() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        run(&mut game, 10.0, NONE);
        let expected = START_SPEED + 10.0 * ACCELERATION;
        assert!((game.speed - expected).abs() < 0.2, "speed {}", game.speed);
        run(&mut game, 200.0, NONE);
        assert_eq!(game.speed, MAX_SPEED);
    }

    #[test]
    fn obstacles_spawn_offscreen_and_scroll_left() {
        let mut game = Game::new(3);
        game.update(DT, JUMP);
        run(&mut game, 1.1, NONE);
        assert_eq!(game.obstacles.len(), 1);
        let x = game.obstacles[0].x;
        assert!(x > 960.0 && x <= SPAWN_X);
        game.update(DT, NONE);
        assert!(game.obstacles[0].x < x);
    }

    #[test]
    fn gaps_leave_room_to_land() {
        let mut game = Game::new(5);
        game.update(DT, JUMP);
        game.rex = Rex::new();
        let mut checked = 0;
        // Ducking keeps the rex low but doesn't stop the crash check, so
        // collisions are cleared by hand to keep the run going.
        for _ in 0..(120.0 / DT) as u32 {
            game.update(DT, DUCK);
            game.state = State::Running;
            for pair in game.obstacles.windows(2) {
                let gap = pair[1].x - pair[0].right();
                assert!(gap >= START_SPEED * MIN_GAP_TIME - 1.0, "gap {gap}");
                checked += 1;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn running_into_a_cactus_ends_the_run() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 300.0, 0.0));
        play(&mut game, |_| NONE);
        assert_eq!(game.state, State::Over(0.0));
    }

    #[test]
    fn jumping_clears_the_widest_cactus_group() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(3), 400.0, 0.0));
        play(&mut game, |ahead| if ahead < 40.0 { JUMP } else { NONE });
        assert_eq!(game.state, State::Running);
    }

    #[test]
    fn ducking_passes_under_a_middle_pterodactyl() {
        let mid = PTERODACTYL_LANES[1];
        let mut game = course(Obstacle::new(Kind::Pterodactyl, 400.0, mid));
        play(&mut game, |_| DUCK);
        assert_eq!(game.state, State::Running);

        let mut game = course(Obstacle::new(Kind::Pterodactyl, 400.0, mid));
        play(&mut game, |_| NONE);
        assert!(matches!(game.state, State::Over(_)));
    }

    #[test]
    fn a_high_pterodactyl_clears_a_standing_rex() {
        let high = PTERODACTYL_LANES[2];
        let mut game = course(Obstacle::new(Kind::Pterodactyl, 400.0, high));
        play(&mut game, |_| NONE);
        assert_eq!(game.state, State::Running);
    }

    #[test]
    fn a_low_pterodactyl_must_be_jumped() {
        let low = PTERODACTYL_LANES[0];
        let mut game = course(Obstacle::new(Kind::Pterodactyl, 400.0, low));
        play(&mut game, |_| DUCK);
        assert!(matches!(game.state, State::Over(_)));

        let mut game = course(Obstacle::new(Kind::Pterodactyl, 400.0, low));
        play(&mut game, |ahead| if ahead < 40.0 { JUMP } else { NONE });
        assert_eq!(game.state, State::Running);
    }

    #[test]
    fn score_follows_chrome() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.distance = 4000.0;
        assert_eq!(game.score(), 100);
        run(&mut game, 1.0, NONE);
        // About 9 points a second at the starting speed.
        assert!((108..=110).contains(&game.score()), "{}", game.score());
    }

    #[test]
    fn the_score_blinks_at_each_hundred() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.distance = 3990.0;
        game.update(DT, NONE);
        assert_eq!(game.milestone_flash, 0.0);
        run(&mut game, 0.1, NONE);
        assert!(game.milestone_flash > 0.0);
        run(&mut game, MILESTONE_FLASH, NONE);
        assert_eq!(game.milestone_flash, 0.0);
    }

    #[test]
    fn a_crash_records_only_a_better_score() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0));
        game.high_score = 500;
        game.distance = 4000.0;
        play(&mut game, |_| NONE);
        assert_eq!(game.high_score, 500);

        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0));
        game.high_score = 50;
        game.distance = 4000.0;
        play(&mut game, |_| NONE);
        let crashed_at = game.score();
        assert!(crashed_at > 100);
        assert_eq!(game.high_score, crashed_at);

        // It survives the restart while the score itself starts over.
        run(&mut game, RESTART_DELAY, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.high_score, crashed_at);
        assert_eq!(game.score(), 0);
    }

    #[test]
    fn the_world_freezes_after_a_crash() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0));
        play(&mut game, |_| NONE);
        let distance = game.distance;
        run(&mut game, 1.0, NONE);
        assert_eq!(game.distance, distance);
    }

    #[test]
    fn restart_needs_a_fresh_press_after_the_delay() {
        // The cactus starts on top of the rex so the jump can't clear it.
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), REX_X, 0.0));
        // Holding jump through the crash must not restart on its own.
        play(&mut game, |_| JUMP);
        assert!(matches!(game.state, State::Over(_)));
        run(&mut game, 1.0, JUMP);
        assert!(matches!(game.state, State::Over(_)));

        // A press too soon after a fresh crash is ignored.
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0));
        play(&mut game, |_| NONE);
        game.update(DT, JUMP);
        assert!(matches!(game.state, State::Over(_)));

        game.update(DT, NONE);
        run(&mut game, RESTART_DELAY, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.state, State::Running);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.speed, START_SPEED);
        assert!(game.obstacles.is_empty());
        // The restarting press doesn't make the new rex jump.
        assert_eq!(game.rex.pose, Pose::Running);
    }
}
