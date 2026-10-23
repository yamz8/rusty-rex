//! Who plays the screensaver: nobody yet, the autopilot, or a person.
//!
//! The screensaver opens on the waiting screen. A game key starts a run for
//! the person at the keyboard; if nobody does within ATTRACT_DELAY, the
//! autopilot plays, and a jump press takes over from it at any moment. A
//! person who walks away hands back to the autopilot, so the screen keeps
//! moving. Nothing here touches the window, so the rules are unit tested.

use crate::game::{Game, State};
use crate::rex::Input;

/// The waiting screen gives a person this long to start before the
/// autopilot plays.
pub const ATTRACT_DELAY: f32 = 10.0;
/// A person's game-over screen with no key pressed this long goes back to
/// the autopilot: they have most likely walked away.
pub const IDLE_AFTER_CRASH: f32 = 15.0;
/// Mid-run, which in practice means paused, the autopilot takes over after
/// this long without a key.
pub const IDLE_IN_RUN: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pilot {
    /// The waiting screen, before anyone has played.
    Waiting,
    Autopilot,
    Person,
}

/// What a frame should do with the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    /// Nobody plays: leave the waiting screen as it is.
    Wait,
    /// The autopilot plays, unpausing the run first if a person left it
    /// paused.
    Autopilot { unpause: bool },
    /// The person's keys play.
    Person,
}

pub struct Attract {
    pub pilot: Pilot,
    /// Seconds since the last game key.
    idle: f32,
    /// The run under way, or the next one, was started by a person. Only
    /// these runs may set the high score: a run the autopilot carried before
    /// a person took over doesn't count.
    persons_run: bool,
}

impl Default for Attract {
    fn default() -> Self {
        Self {
            pilot: Pilot::Waiting,
            idle: 0.0,
            persons_run: false,
        }
    }
}

impl Attract {
    /// Decides who plays this frame. `input` is the person's keys; `touched`
    /// says whether they pressed any game key at all. Sets whether the run
    /// may record a high score.
    pub fn step(&mut self, game: &mut Game, elapsed: f32, input: Input, touched: bool) -> Drive {
        self.idle = if touched { 0.0 } else { self.idle + elapsed };
        // Space, Up or a click: the same press that starts a run.
        let takes_over = input.jump_pressed;
        let between_runs = matches!(game.state, State::Waiting | State::Over(_));

        match self.pilot {
            Pilot::Waiting if takes_over => self.pilot = Pilot::Person,
            Pilot::Waiting if self.idle >= ATTRACT_DELAY => self.pilot = Pilot::Autopilot,
            Pilot::Autopilot if takes_over => self.pilot = Pilot::Person,
            Pilot::Person => {
                let limit = if matches!(game.state, State::Over(_)) {
                    IDLE_AFTER_CRASH
                } else {
                    IDLE_IN_RUN
                };
                if self.idle >= limit {
                    self.pilot = Pilot::Autopilot;
                }
            }
            _ => {}
        }

        // Whoever starts the next run owns it; a run taken over midway
        // stays the autopilot's.
        if between_runs {
            self.persons_run = self.pilot == Pilot::Person;
        }
        game.recording = self.persons_run;

        match self.pilot {
            Pilot::Waiting => Drive::Wait,
            Pilot::Autopilot => Drive::Autopilot {
                unpause: game.state == State::Paused,
            },
            Pilot::Person => Drive::Person,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autopilot;
    use crate::clock::TICK;
    use crate::game::RESTART_DELAY;

    const JUMP: Input = Input {
        jump_pressed: true,
        jump: true,
        duck: false,
        restart: false,
    };

    /// Steps `seconds` of frames, playing as `step` says, with the person
    /// pressing nothing.
    fn idle(attract: &mut Attract, game: &mut Game, seconds: f32) {
        for _ in 0..(seconds / TICK).round() as u32 {
            play(attract, game, Input::default(), false);
        }
    }

    fn play(attract: &mut Attract, game: &mut Game, input: Input, touched: bool) {
        match attract.step(game, TICK, input, touched) {
            Drive::Wait => {}
            Drive::Autopilot { unpause } => {
                if unpause {
                    game.toggle_pause();
                }
                let input = autopilot::input(game, TICK);
                game.update(TICK, input);
            }
            Drive::Person => game.update(TICK, input),
        }
    }

    #[test]
    fn it_waits_for_a_person_then_the_autopilot_plays() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        idle(&mut attract, &mut game, ATTRACT_DELAY - 1.0);
        assert_eq!(attract.pilot, Pilot::Waiting);
        assert_eq!(game.state, State::Waiting);
        idle(&mut attract, &mut game, 2.0);
        assert_eq!(attract.pilot, Pilot::Autopilot);
        assert_ne!(game.state, State::Waiting);
    }

    #[test]
    fn a_jump_on_the_waiting_screen_starts_a_persons_run() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        idle(&mut attract, &mut game, 3.0);
        play(&mut attract, &mut game, JUMP, true);
        assert_eq!(attract.pilot, Pilot::Person);
        assert!(matches!(game.state, State::Intro(_)));
        assert!(game.recording);
    }

    #[test]
    fn a_jump_takes_over_from_the_autopilot_but_the_run_stays_its() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        idle(&mut attract, &mut game, ATTRACT_DELAY + 5.0);
        assert_eq!(game.state, State::Running);
        play(&mut attract, &mut game, JUMP, true);
        assert_eq!(attract.pilot, Pilot::Person);
        // the person plays from here, but the autopilot carried the run
        assert!(!game.recording);
    }

    #[test]
    fn down_alone_does_not_take_over() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        idle(&mut attract, &mut game, ATTRACT_DELAY + 1.0);
        let duck = Input {
            duck: true,
            ..Input::default()
        };
        play(&mut attract, &mut game, duck, true);
        assert_eq!(attract.pilot, Pilot::Autopilot);
    }

    #[test]
    fn only_runs_a_person_started_set_the_high_score() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        // The autopilot plays a long run and never crashes; force a crash by
        // putting a cactus on the rex, and the high score stays as it was.
        idle(&mut attract, &mut game, ATTRACT_DELAY + 30.0);
        let score = game.score();
        assert!(score > 100);
        crash(&mut attract, &mut game, Input::default(), false);
        assert_eq!(game.high_score, 0, "an autopilot run set the high score");

        // A person takes over at the game-over screen and starts a run of
        // their own: its crash counts.
        idle(&mut attract, &mut game, 0.5);
        for _ in 0..(RESTART_DELAY / TICK) as u32 + 2 {
            play(&mut attract, &mut game, Input::default(), true);
        }
        play(&mut attract, &mut game, JUMP, true);
        assert_eq!(attract.pilot, Pilot::Person);
        assert_eq!(game.state, State::Running);
        for _ in 0..(20.0 / TICK) as u32 {
            play(&mut attract, &mut game, Input::default(), true);
            if game.state != State::Running {
                break;
            }
        }
        if game.state == State::Running {
            crash(&mut attract, &mut game, Input::default(), true);
        }
        assert!(game.high_score > 0, "a person's run didn't count");
    }

    /// Puts a large cactus on the runner, so the next tick crashes.
    fn crash(attract: &mut Attract, game: &mut Game, input: Input, touched: bool) {
        use crate::game::REX_X;
        use crate::obstacle::{Kind, Obstacle};
        game.rex = crate::rex::Rex::new();
        game.obstacles = vec![Obstacle::new(Kind::LargeCactus(1), REX_X, 0.0)];
        play(attract, game, input, touched);
        assert!(matches!(game.state, State::Over(_)));
    }

    #[test]
    fn a_person_who_walks_away_hands_back_to_the_autopilot() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        play(&mut attract, &mut game, JUMP, true);
        // past the intro, where nothing can be hit yet
        idle(&mut attract, &mut game, 1.5);
        crash(&mut attract, &mut game, Input::default(), true);
        idle(&mut attract, &mut game, IDLE_AFTER_CRASH - 1.0);
        assert_eq!(attract.pilot, Pilot::Person);
        assert!(matches!(game.state, State::Over(_)));
        idle(&mut attract, &mut game, 2.0);
        assert_eq!(attract.pilot, Pilot::Autopilot);
        // the autopilot plays again, and its run doesn't record
        idle(&mut attract, &mut game, autopilot::RESTART_PAUSE);
        assert_eq!(game.state, State::Running);
        assert!(!game.recording);
    }

    #[test]
    fn a_run_left_paused_goes_back_to_the_autopilot() {
        let (mut attract, mut game) = (Attract::default(), Game::new(1));
        play(&mut attract, &mut game, JUMP, true);
        idle(&mut attract, &mut game, 1.0);
        game.state = State::Running;
        game.toggle_pause();
        idle(&mut attract, &mut game, IDLE_IN_RUN - 1.0);
        assert_eq!(game.state, State::Paused);
        idle(&mut attract, &mut game, 2.0);
        assert_eq!(attract.pilot, Pilot::Autopilot);
        assert_eq!(game.state, State::Running);
    }
}
