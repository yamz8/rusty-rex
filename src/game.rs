//! The running game: the world scrolling past the rex, getting faster, with
//! obstacles to clear.
//!
//! Like the rex, this never touches the window or GL. Horizontal positions are in
//! playfield pixels and heights in pixels above the ground.

use crate::obstacle::{self, Hitbox, Kind, Obstacle, Rng, SpriteBox};
use crate::rex::{Input, Pose, Rex, STANDING_SIZE};

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
/// Chrome sends no obstacles for the first 3 seconds of a run.
const CLEAR_TIME: f32 = 3.0;
/// Obstacles this far left of the playfield are gone for good.
pub const DESPAWN_X: f32 = -300.0;
/// After a crash, jump is ignored this long so a panicked press doesn't skip
/// straight past the game-over screen.
pub const RESTART_DELAY: f32 = 0.5;
/// Chrome's score is the distance in whole pixels times this, rounded.
const SCORE_PER_PIXEL: f32 = 0.025;
/// Every this many points the score blinks, as in Chrome.
pub const MILESTONE: u32 = 100;
/// Chrome blinks the score off and on four times, a quarter second each way.
pub const MILESTONE_FLASH: f32 = 2.0;
pub const FLASH_PERIOD: f32 = 0.5;
/// Chrome brings on the night every this many points.
pub const NIGHT_EVERY: u32 = 700;
/// How long each night lasts before day returns, as in Chrome.
const NIGHT_LENGTH: f32 = 12.0;
/// How long the change between day and night takes.
const NIGHT_FADE_TIME: f32 = 1.0;
/// Chrome's moon goes through this many phases, one each night.
pub const MOON_PHASES: usize = 7;
/// Chrome's collision boxes for the rex, standing and ducking. Both are in
/// the standing sprite's 47 px frame, with the duck at its bottom.
const REX_RUNNING_BOXES: [SpriteBox; 6] = [
    (22.0, 0.0, 17.0, 16.0),
    (1.0, 18.0, 30.0, 9.0),
    (10.0, 35.0, 14.0, 8.0),
    (1.0, 24.0, 29.0, 5.0),
    (5.0, 30.0, 21.0, 4.0),
    (9.0, 34.0, 15.0, 4.0),
];
const REX_DUCKING_BOXES: [SpriteBox; 1] = [(1.0, 18.0, 55.0, 25.0)];

/// Things that happened during an update, for the sound effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Jump,
    Milestone,
    Crash,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Before the first jump, as in Chrome.
    Waiting,
    Running,
    /// Frozen mid-run until the player resumes.
    Paused,
    /// Crashed; holds the seconds since the crash.
    Over(f32),
}

/// Clone lets the autopilot try moves on copies of a run.
#[derive(Clone, Debug)]
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
    /// The milestone the score shows while it blinks.
    milestone_score: u32,
    /// 0 is full day, 1 full night; in between while fading.
    pub night: f32,
    /// Seconds of night left; day returns when it runs out.
    night_left: f32,
    /// Nights so far, across runs, so the moon keeps moving through its
    /// phases instead of starting over with every restart.
    nights: u32,
    /// Events since the caller last drained them.
    pub events: Vec<Event>,
    rng: Rng,
    /// Seconds since this run started, for Chrome's clear time.
    running_time: f32,
    /// The last two kinds spawned, newest last.
    recent: Vec<Kind>,
    /// Jump was held last update. Starting, restarting and take-off need a
    /// fresh press, otherwise holding the key would bounce the rex the moment
    /// it lands. This also sees the key while paused, so a key held through
    /// a pause doesn't count as a press on resuming.
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
            milestone_score: 0,
            night: 0.0,
            night_left: 0.0,
            nights: 0,
            events: Vec::new(),
            rng: Rng::new(seed),
            running_time: 0.0,
            recent: Vec::new(),
            jump_was_down: false,
        }
    }

    pub fn update(&mut self, dt: f32, input: Input) {
        let input = Input {
            jump_pressed: input.jump_pressed || (input.jump && !self.jump_was_down),
            ..input
        };
        self.jump_was_down = input.jump;

        match self.state {
            State::Waiting => {
                if input.jump_pressed {
                    self.state = State::Running;
                    self.run(dt, input);
                }
            }
            State::Running => self.run(dt, input),
            State::Paused => {}
            State::Over(since) => {
                let since = since + dt;
                self.state = State::Over(since);
                if input.jump_pressed && since >= RESTART_DELAY {
                    self.restart();
                }
            }
        }
    }

    /// Pauses a run or resumes a paused one. Does nothing before the run
    /// starts or after a crash, where there is nothing to freeze.
    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            State::Running => State::Paused,
            State::Paused => State::Running,
            other => other,
        };
    }

    pub fn score(&self) -> u32 {
        (self.distance.ceil() * SCORE_PER_PIXEL).round() as u32
    }

    /// The score to display: it holds at the milestone while it blinks.
    pub fn shown_score(&self) -> u32 {
        if self.milestone_flash > 0.0 {
            self.milestone_score
        } else {
            self.score()
        }
    }

    /// Whether the blinking score is in its off half.
    pub fn score_hidden(&self) -> bool {
        self.milestone_flash > 0.0
            && (MILESTONE_FLASH - self.milestone_flash) % FLASH_PERIOD < FLASH_PERIOD / 2.0
    }

    /// Tonight's moon phase, counting from the first night's.
    pub fn moon_phase(&self) -> usize {
        self.nights.saturating_sub(1) as usize % MOON_PHASES
    }

    /// Whether the rex touches an obstacle, tested as Chrome does: a rough
    /// box first, then each of the rex's boxes against each of the
    /// obstacle's. The rough box is always the standing rex's, as in Chrome.
    fn crashed(&self) -> bool {
        let (w, h) = STANDING_SIZE;
        let outline = Hitbox::outline(REX_X, self.rex.height, w, h);
        let boxes: &[SpriteBox] = if self.rex.pose == Pose::Ducking {
            &REX_DUCKING_BOXES
        } else {
            &REX_RUNNING_BOXES
        };
        let mut rex = [outline; REX_RUNNING_BOXES.len()];
        for (placed, &b) in rex.iter_mut().zip(boxes) {
            *placed = Hitbox::place(b, REX_X, self.rex.height, h);
        }
        let rex = &rex[..boxes.len()];
        self.obstacles
            .iter()
            .any(|o| o.outline().overlaps(&outline) && o.hits(rex))
    }

    fn run(&mut self, dt: f32, input: Input) {
        let was_jumping = self.rex.pose == Pose::Jumping;
        self.rex.update(dt, input, self.speed);
        if !was_jumping && self.rex.pose == Pose::Jumping {
            self.events.push(Event::Jump);
        }

        let step = self.speed * dt;
        let before = self.score();
        self.distance += step;
        self.milestone_flash = (self.milestone_flash - dt).max(0.0);
        // Chrome ignores milestones while one is still blinking.
        if self.score() / MILESTONE > before / MILESTONE && self.milestone_flash == 0.0 {
            self.milestone_flash = MILESTONE_FLASH;
            self.milestone_score = self.score() / MILESTONE * MILESTONE;
            self.events.push(Event::Milestone);
        }
        self.night_left = (self.night_left - dt).max(0.0);
        if self.score() / NIGHT_EVERY > before / NIGHT_EVERY {
            self.night_left = NIGHT_LENGTH;
            self.nights += 1;
        }
        let target = if self.night_left > 0.0 { 1.0 } else { 0.0 };
        let fade_step = dt / NIGHT_FADE_TIME;
        self.night += (target - self.night).clamp(-fade_step, fade_step);
        self.speed = (self.speed + ACCELERATION * dt).min(MAX_SPEED);

        for o in &mut self.obstacles {
            o.x -= step + o.speed_offset * dt;
            o.age += dt;
        }
        self.obstacles.retain(|o| o.right() > DESPAWN_X);

        // As in Chrome, the next obstacle comes once the last one is its gap
        // clear of where obstacles appear.
        self.running_time += dt;
        let due = match self.obstacles.last() {
            Some(last) => last.right() + last.gap < SPAWN_X,
            None => self.running_time > CLEAR_TIME,
        };
        if due {
            self.spawn();
        }

        if self.crashed() {
            self.state = State::Over(0.0);
            self.milestone_flash = 0.0;
            self.events.push(Event::Crash);
            self.high_score = self.high_score.max(self.score());
        }
    }

    fn spawn(&mut self) {
        let mut o = obstacle::choose(&mut self.rng, self.speed, &self.recent, SPAWN_X);
        o.gap = o.choose_gap(self.speed, &mut self.rng);
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
            nights: self.nights,
            ..Self::new(0)
        };
        self.jump_was_down = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obstacle::{PTERODACTYL_LANES, PTERODACTYL_SPEED_OFFSET};
    use crate::rex::Pose;

    const DT: f32 = 1.0 / 120.0;
    const NONE: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: false,
    };
    const JUMP: Input = Input {
        jump_pressed: false,
        jump: true,
        duck: false,
    };
    const DUCK: Input = Input {
        jump_pressed: false,
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
        // Nothing follows it.
        game.obstacles = vec![Obstacle {
            gap: f32::INFINITY,
            ..obstacle
        }];
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
        run(&mut game, CLEAR_TIME - 0.1, NONE);
        assert!(game.obstacles.is_empty(), "obstacles before the clear time");
        run(&mut game, 0.2, NONE);
        assert_eq!(game.obstacles.len(), 1);
        let x = game.obstacles[0].x;
        assert!(x > 960.0 && x <= SPAWN_X);
        game.update(DT, NONE);
        assert!(game.obstacles[0].x < x);
    }

    #[test]
    fn each_obstacle_comes_its_gap_after_the_last() {
        let mut game = Game::new(5);
        game.update(DT, JUMP);
        game.rex = Rex::new();
        let mut checked = 0;
        let mut count = game.obstacles.len();
        // Collisions are cleared by hand to keep the run going until top
        // speed, where gaps are tightest.
        for _ in 0..(150.0 / DT) as u32 {
            game.update(DT, DUCK);
            game.state = State::Running;
            if game.obstacles.len() > count && game.obstacles.len() >= 2 {
                let [.., before, new] = &game.obstacles[..] else {
                    unreachable!()
                };
                // It appears on the tick the gap opens, so the space is the
                // gap plus at most one tick of movement.
                let space = new.x - before.right();
                let tick = (game.speed + PTERODACTYL_SPEED_OFFSET) * DT;
                assert!(
                    (before.gap..before.gap + tick + 0.01).contains(&space),
                    "space {space}, gap {}",
                    before.gap
                );
                checked += 1;
            }
            count = game.obstacles.len();
        }
        assert_eq!(game.speed, MAX_SPEED);
        assert!(checked > 50, "only {checked} gaps checked");
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
        // Whole pixels, rounded: 19.2 px is 20, which is half a point.
        game.distance = 19.2;
        assert_eq!(game.score(), 1);
        game.distance = 19.0;
        assert_eq!(game.score(), 0);
    }

    #[test]
    fn the_score_blinks_at_each_hundred() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.distance = 3970.0;
        game.update(DT, NONE);
        assert_eq!(game.milestone_flash, 0.0);
        run(&mut game, 0.1, NONE);
        assert!(game.milestone_flash > 0.0);
        // It starts hidden and holds the milestone while the run goes on.
        assert!(game.score_hidden());
        run(&mut game, 0.3, NONE);
        assert!(!game.score_hidden());
        assert_eq!(game.shown_score(), 100);
        assert!(game.score() > 100);
        run(&mut game, MILESTONE_FLASH, NONE);
        assert_eq!(game.milestone_flash, 0.0);
        assert_eq!(game.shown_score(), game.score());
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
    fn night_falls_every_seven_hundred_points_for_twelve_seconds() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.distance = 699.0 / 0.025;
        game.update(DT, NONE);
        assert_eq!(game.night, 0.0);
        run(&mut game, 0.5, NONE);
        assert!(game.night > 0.0 && game.night < 1.0, "{}", game.night);
        run(&mut game, 1.0, NONE);
        assert_eq!(game.night, 1.0);

        // Still night near the end of the twelve seconds, day soon after,
        // long before the next 700 points.
        run(&mut game, NIGHT_LENGTH - 2.0, NONE);
        assert_eq!(game.night, 1.0);
        run(&mut game, 2.0 + NIGHT_FADE_TIME, NONE);
        assert_eq!(game.night, 0.0);
        assert!(game.score() < 2 * NIGHT_EVERY);

        // The next 700 brings the next night.
        game.distance = (2 * NIGHT_EVERY) as f32 / 0.025 - 30.0;
        run(&mut game, 1.5, NONE);
        assert_eq!(game.night, 1.0);
    }

    #[test]
    fn the_moon_moves_on_a_phase_each_night_and_across_restarts() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        let mut phases = Vec::new();
        for night in 1..=MOON_PHASES as u32 + 1 {
            game.distance = (night * NIGHT_EVERY) as f32 / 0.025 - 30.0;
            run(&mut game, 0.1, NONE);
            phases.push(game.moon_phase());
        }
        let expected: Vec<usize> = (0..MOON_PHASES).chain([0]).collect();
        assert_eq!(phases, expected);

        // A crash and restart doesn't send the moon back to its first phase.
        let phase = game.moon_phase();
        game.obstacles = vec![Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0)];
        play(&mut game, |_| NONE);
        run(&mut game, RESTART_DELAY, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.moon_phase(), phase);
    }

    #[test]
    fn restarting_brings_back_the_day() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 200.0, 0.0));
        game.night = 1.0;
        game.distance = 700.0 / 0.025;
        play(&mut game, |_| NONE);
        run(&mut game, RESTART_DELAY, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.night, 0.0);
    }

    #[test]
    fn reports_jumps_milestones_and_crashes() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 300.0, 0.0));
        game.events.clear();
        game.distance = 99.0 / 0.025;
        game.update(DT, JUMP);
        // Holding jump must not report more jumps.
        run(&mut game, 0.2, JUMP);
        assert_eq!(game.events, vec![Event::Jump, Event::Milestone]);

        game.events.clear();
        play(&mut game, |_| NONE);
        assert_eq!(game.events, vec![Event::Crash]);
    }

    #[test]
    fn holding_jump_does_not_bounce_after_landing() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.events.clear();
        run(&mut game, 2.0, JUMP);
        assert_eq!(game.rex.pose, Pose::Running);
        assert_eq!(game.events, vec![Event::Jump]);

        game.update(DT, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.rex.pose, Pose::Jumping);
    }

    #[test]
    fn pausing_freezes_the_run_until_resumed() {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), 300.0, 0.0));
        game.toggle_pause();
        assert_eq!(game.state, State::Paused);
        let (distance, x) = (game.distance, game.obstacles[0].x);
        run(&mut game, 2.0, JUMP);
        assert_eq!(game.distance, distance);
        assert_eq!(game.obstacles[0].x, x);
        assert_eq!(game.rex.pose, Pose::Running);

        game.toggle_pause();
        assert_eq!(game.state, State::Running);
        game.update(DT, NONE);
        assert!(game.distance > distance);
    }

    #[test]
    fn jump_held_through_a_pause_needs_a_fresh_press() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.toggle_pause();
        run(&mut game, 0.5, JUMP);
        game.toggle_pause();
        run(&mut game, 0.2, JUMP);
        assert_eq!(game.rex.pose, Pose::Running);

        game.update(DT, NONE);
        game.update(DT, JUMP);
        assert_eq!(game.rex.pose, Pose::Jumping);
    }

    #[test]
    fn only_a_running_game_can_pause() {
        let mut game = Game::new(1);
        game.toggle_pause();
        assert_eq!(game.state, State::Waiting);

        let mut game = course(Obstacle::new(Kind::LargeCactus(1), REX_X, 0.0));
        game.update(DT, NONE);
        game.toggle_pause();
        assert!(matches!(game.state, State::Over(_)));
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
