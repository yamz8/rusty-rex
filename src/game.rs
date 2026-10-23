//! The running game: the world scrolling past the rex, getting faster, with
//! obstacles to clear.
//!
//! Like the rex, this never touches the window or GL. Horizontal positions are in
//! playfield pixels and heights in pixels above the ground.

use crate::obstacle::{self, Hitbox, Kind, Obstacle, Rng};
use crate::rex::{Input, Pose, Rex};
use crate::runner::{FRAME_HEIGHT, Runner};

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
/// A pterodactyl's own speed only counts from here on. Chrome's appear at
/// the canvas edge, 550 px ahead of the rex; ours start further out, and
/// flying at their own speed all that way would close or widen their gaps
/// twice as much as in Chrome.
pub const DRIFT_FROM: f32 = REX_X + 550.0;
/// Chrome sends no obstacles for the first 3 seconds of a run.
const CLEAR_TIME: f32 = 3.0;
/// Obstacles this far left of the playfield are gone for good.
pub const DESPAWN_X: f32 = -300.0;
/// After a crash, jump is ignored this long so a panicked press doesn't skip
/// straight past the game-over screen; Enter or a click restarts at once.
pub const RESTART_DELAY: f32 = 1.2;
/// After the first jump lands, Chrome widens the game out from the rex over
/// this long before the world starts moving.
pub const INTRO_TIME: f32 = 0.4;
/// Meanwhile the rex walks in from this far left of where it runs, at
/// Chrome's 50 px per 1.5 s.
pub const INTRO_WALK: f32 = 50.0 / 1.5 * INTRO_TIME;
/// Clicking the high score at game over makes it blink this long, seven
/// times; a second click meanwhile resets it.
pub const HIGH_SCORE_FLASH: f32 = 7.0 * FLASH_PERIOD;
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
/// How long the change between day and night takes: Chrome's 1.5 s page
/// transition, eased by NIGHT_EASE.
const NIGHT_FADE_TIME: f32 = 1.5;
/// Chrome's cubic-bezier(0.65, 0.05, 0.36, 1) for that transition.
const NIGHT_EASE: [f32; 4] = [0.65, 0.05, 0.36, 1.0];
/// Clouds drift at Chrome's steady 1 px/frame, whatever the game speed.
pub const CLOUD_SPEED: f32 = 60.0;
/// Chrome's moon goes through this many phases, one each night.
pub const MOON_PHASES: usize = 7;

/// Things that happened during an update, for the sound effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Jump,
    Milestone,
    Crash,
    /// A new run after a crash; Chrome plays its button sound.
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Before the first jump, as in Chrome.
    Waiting,
    /// The first jump and the opening of the game, before the world moves.
    /// Holds the seconds since the rex landed.
    Intro(f32),
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
    /// Who runs; the physics are the same, the size and boxes not.
    pub runner: Runner,
    pub rex: Rex,
    /// Scroll speed in pixels per second.
    pub speed: f32,
    /// How far the world has scrolled in this run.
    pub distance: f32,
    pub obstacles: Vec<Obstacle>,
    /// Best score so far, including earlier sessions once loaded from disk.
    pub high_score: u32,
    /// Seconds of high score blinking left after a click on it.
    pub high_score_flash: f32,
    /// Seconds of score blinking left after the last milestone.
    pub milestone_flash: f32,
    /// The milestone the score shows while it blinks.
    milestone_score: u32,
    /// 0 is full day, 1 full night; in between while fading, eased.
    pub night: f32,
    /// How far through the fade to night, before easing.
    night_fade: f32,
    /// Seconds the night sky has been up, across nights, which moves the
    /// moon and stars.
    pub sky_time: f32,
    /// How far the clouds have drifted, kept across runs as in Chrome.
    pub clouds: f32,
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
            runner: Runner::Rex,
            rex: Rex::new(),
            speed: START_SPEED,
            distance: 0.0,
            obstacles: Vec::new(),
            high_score: 0,
            high_score_flash: 0.0,
            milestone_flash: 0.0,
            milestone_score: 0,
            night: 0.0,
            night_fade: 0.0,
            sky_time: 0.0,
            clouds: 0.0,
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
                    self.state = State::Intro(0.0);
                    self.intro(dt, input);
                }
            }
            State::Intro(_) => self.intro(dt, input),
            State::Running => self.run(dt, input),
            State::Paused => {}
            State::Over(since) => {
                let since = since + dt;
                self.state = State::Over(since);
                self.high_score_flash = (self.high_score_flash - dt).max(0.0);
                if input.restart || (input.jump_pressed && since >= RESTART_DELAY) {
                    self.restart();
                }
            }
        }
    }

    /// Whether the game is moving on its own, so time should pass in fixed
    /// ticks.
    pub fn is_live(&self) -> bool {
        matches!(self.state, State::Running | State::Intro(_))
    }

    /// The first jump happens in place; once the rex lands, the game opens
    /// out and the run begins.
    fn intro(&mut self, dt: f32, input: Input) {
        self.step_rex(dt, input);
        if let State::Intro(landed) = self.state
            && self.rex.on_ground()
        {
            let landed = landed + dt;
            self.state = if landed >= INTRO_TIME {
                State::Running
            } else {
                State::Intro(landed)
            };
        }
    }

    /// How much of the game shows, from 0 (only the rex) to 1, easing out
    /// as Chrome's opening animation does.
    pub fn reveal(&self) -> f32 {
        match self.state {
            State::Waiting => 0.0,
            State::Intro(t) => 1.0 - (1.0 - t / INTRO_TIME).powi(2),
            _ => 1.0,
        }
    }

    /// Where the rex is drawn: it walks in during the intro.
    pub fn rex_x(&self) -> f32 {
        REX_X - INTRO_WALK * (1.0 - self.reveal())
    }

    /// A click on the high score at game over: the first makes it blink, a
    /// second while it blinks resets it to zero, as in Chrome.
    pub fn click_high_score(&mut self) {
        if !matches!(self.state, State::Over(_)) || self.high_score == 0 {
            return;
        }
        if self.high_score_flash > 0.0 {
            self.high_score = 0;
            self.high_score_flash = 0.0;
        } else {
            self.high_score_flash = HIGH_SCORE_FLASH;
        }
    }

    /// Whether the blinking high score is in its off half.
    pub fn high_score_hidden(&self) -> bool {
        blink_off(HIGH_SCORE_FLASH, self.high_score_flash)
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
        blink_off(MILESTONE_FLASH, self.milestone_flash)
    }

    /// Tonight's moon phase, counting from the first night's.
    pub fn moon_phase(&self) -> usize {
        self.nights.saturating_sub(1) as usize % MOON_PHASES
    }

    /// Whether the runner touches an obstacle, tested as Chrome does: a
    /// rough box first, then each of the runner's boxes against each of the
    /// obstacle's. The rough box is always the standing one, as in Chrome.
    fn crashed(&self) -> bool {
        let (w, _) = self.runner.standing();
        let h = FRAME_HEIGHT;
        let outline = Hitbox::outline(REX_X, self.rex.height, w, h);
        let boxes = self.runner.boxes(self.rex.pose);
        let mut rex = [outline; Runner::MAX_BOXES];
        for (placed, &b) in rex.iter_mut().zip(boxes) {
            *placed = Hitbox::place(b, REX_X, self.rex.height, h);
        }
        let rex = &rex[..boxes.len()];
        self.obstacles
            .iter()
            .any(|o| o.outline().overlaps(&outline) && o.hits(rex))
    }

    /// Moves the rex, reporting a take-off for the jump sound.
    fn step_rex(&mut self, dt: f32, input: Input) {
        let was_jumping = self.rex.pose == Pose::Jumping;
        self.rex.update(dt, input, self.speed);
        if !was_jumping && self.rex.pose == Pose::Jumping {
            self.events.push(Event::Jump);
        }
    }

    fn run(&mut self, dt: f32, input: Input) {
        self.step_rex(dt, input);

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
        self.night_fade += (target - self.night_fade).clamp(-fade_step, fade_step);
        self.night = cubic_bezier(NIGHT_EASE, self.night_fade);
        if self.night > 0.0 {
            self.sky_time += dt;
        }
        self.clouds += CLOUD_SPEED * dt;
        self.speed = (self.speed + ACCELERATION * dt).min(MAX_SPEED);

        for o in &mut self.obstacles {
            let own = if o.x < DRIFT_FROM {
                o.speed_offset
            } else {
                0.0
            };
            o.x -= step + own * dt;
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
            runner: self.runner,
            nights: self.nights,
            sky_time: self.sky_time,
            clouds: self.clouds,
            ..Self::new(0)
        };
        self.jump_was_down = true;
        self.events.push(Event::Restart);
    }
}

/// Whether a blink lasting `total` seconds, with `left` to go, is in the off
/// half of its period. Chrome's blinks start off.
fn blink_off(total: f32, left: f32) -> bool {
    left > 0.0 && (total - left) % FLASH_PERIOD < FLASH_PERIOD / 2.0
}

/// The CSS easing curve with control points `[x1, y1, x2, y2]`, at `p`
/// of the way through.
fn cubic_bezier([x1, y1, x2, y2]: [f32; 4], p: f32) -> f32 {
    // The ends are exact, and most of the time is spent at one of them.
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let at = |a: f32, b: f32, t: f32| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    // x rises with t, so halving finds the t for this p.
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if at(x1, x2, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    at(y1, y2, (lo + hi) / 2.0).clamp(0.0, 1.0)
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
        restart: false,
    };
    const JUMP: Input = Input {
        jump_pressed: false,
        jump: true,
        duck: false,
        restart: false,
    };
    const DUCK: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: true,
        restart: false,
    };

    fn run(game: &mut Game, seconds: f32, input: Input) {
        for _ in 0..(seconds / DT).round() as u32 {
            game.update(DT, input);
        }
    }

    /// A running game with no random spawns, holding just `obstacle`.
    fn course(obstacle: Obstacle) -> Game {
        let mut game = Game::new(1);
        game.state = State::Running;
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
        assert_eq!(game.state, State::Intro(0.0));
        assert_eq!(game.rex.pose, Pose::Jumping);
    }

    #[test]
    fn the_world_waits_for_the_intro() {
        let mut game = Game::new(1);
        game.update(DT, JUMP);
        assert_eq!(game.reveal(), 0.0);
        // The first jump lands in place, then the game opens out.
        run(&mut game, 0.7, NONE);
        assert!(matches!(game.state, State::Intro(t) if t > 0.0));
        assert!(game.reveal() > 0.0 && game.reveal() < 1.0);
        assert!(game.rex_x() < REX_X);
        assert_eq!(game.distance, 0.0);
        run(&mut game, INTRO_TIME, NONE);
        assert_eq!(game.state, State::Running);
        assert_eq!((game.reveal(), game.rex_x()), (1.0, REX_X));
        run(&mut game, 0.1, NONE);
        assert!(game.distance > 0.0);
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
        game.state = State::Running;
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
    fn pterodactyls_fly_their_own_speed_only_over_chromes_distance() {
        let far = Obstacle {
            speed_offset: PTERODACTYL_SPEED_OFFSET,
            ..Obstacle::new(Kind::Pterodactyl, SPAWN_X, 50.0)
        };
        let mut game = course(far);
        let before = game.obstacles[0].x;
        game.update(DT, NONE);
        let moved = before - game.obstacles[0].x;
        assert!((moved - game.speed * DT).abs() < 0.01, "{moved}");

        game.obstacles[0].x = DRIFT_FROM - 100.0;
        let before = game.obstacles[0].x;
        game.update(DT, NONE);
        let moved = before - game.obstacles[0].x;
        let expected = (game.speed + PTERODACTYL_SPEED_OFFSET) * DT;
        assert!((moved - expected).abs() < 0.01, "{moved}");
    }

    #[test]
    fn each_obstacle_comes_its_gap_after_the_last() {
        let mut game = Game::new(5);
        game.state = State::Running;
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
    fn the_night_fade_eases_in_and_out() {
        let ease = |p| cubic_bezier(NIGHT_EASE, p);
        assert!(ease(0.0) < 1e-3);
        assert!(ease(1.0) > 1.0 - 1e-3);
        // Slow at the start, quick through the middle.
        assert!(ease(0.2) < 0.1);
        assert!((ease(0.5) - 0.5).abs() < 0.15);
        let mut last = 0.0;
        for i in 1..=100 {
            let y = ease(i as f32 / 100.0);
            assert!(y >= last);
            last = y;
        }
    }

    #[test]
    fn clouds_drift_at_a_steady_pace_and_the_night_sky_moves() {
        let mut slow = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        let mut fast = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        fast.speed = MAX_SPEED;
        run(&mut slow, 1.0, NONE);
        run(&mut fast, 1.0, NONE);
        assert!((slow.clouds - CLOUD_SPEED).abs() < 1.0);
        assert!((fast.clouds - slow.clouds).abs() < 1e-3);
        assert_eq!(slow.sky_time, 0.0, "no night yet");
    }

    #[test]
    fn night_falls_every_seven_hundred_points_for_twelve_seconds() {
        let mut game = course(Obstacle::new(Kind::SmallCactus(1), 1e9, 0.0));
        game.distance = 699.0 / 0.025;
        game.update(DT, NONE);
        assert_eq!(game.night, 0.0);
        run(&mut game, 0.75, NONE);
        assert!(game.night > 0.0 && game.night < 1.0, "{}", game.night);
        run(&mut game, 1.0, NONE);
        assert!(game.night > 0.999, "{}", game.night);
        assert!(game.sky_time > 0.0);

        // Still night near the end of the twelve seconds, day soon after,
        // long before the next 700 points.
        run(&mut game, NIGHT_LENGTH - 2.0, NONE);
        assert!(game.night > 0.999, "{}", game.night);
        run(&mut game, 2.0 + NIGHT_FADE_TIME, NONE);
        assert!(game.night < 1e-3, "{}", game.night);
        assert!(game.score() < 2 * NIGHT_EVERY);

        // The next 700 brings the next night.
        game.distance = (2 * NIGHT_EVERY) as f32 / 0.025 - 30.0;
        run(&mut game, 2.0, NONE);
        assert!(game.night > 0.999, "{}", game.night);
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

    fn crashed() -> Game {
        let mut game = course(Obstacle::new(Kind::LargeCactus(1), REX_X, 0.0));
        play(&mut game, |_| NONE);
        assert!(matches!(game.state, State::Over(_)));
        game.events.clear();
        game
    }

    #[test]
    fn enter_or_a_click_restarts_at_once() {
        let mut game = crashed();
        game.update(
            DT,
            Input {
                restart: true,
                ..NONE
            },
        );
        assert_eq!(game.state, State::Running);
        assert_eq!(game.events, [Event::Restart]);
    }

    #[test]
    fn a_second_click_on_the_high_score_resets_it() {
        let mut game = crashed();
        game.high_score = 900;
        game.click_high_score();
        assert!(game.high_score_flash > 0.0);
        run(&mut game, 0.1, NONE);
        assert!(game.high_score_hidden());
        game.click_high_score();
        assert_eq!(game.high_score, 0);

        // Once the blinking stops, a click starts it again instead.
        let mut game = crashed();
        game.high_score = 900;
        game.click_high_score();
        run(&mut game, HIGH_SCORE_FLASH, NONE);
        game.click_high_score();
        assert_eq!(game.high_score, 900);
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
