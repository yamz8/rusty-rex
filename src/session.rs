//! Bridges window frames and input events to fixed physics ticks.

use crate::attract::Drive;
use crate::autopilot;
use crate::clock::{Clock, TICK};
use crate::game::{Game, State};
use crate::rex::Input;

#[derive(Default)]
pub struct Session {
    clock: Clock,
    input: Input,
}

impl Session {
    pub fn advance(&mut self, game: &mut Game, elapsed: f32, input: Input, pause: bool) {
        // A frame can have no physics tick. Preserve its press even if the
        // key is released before the next tick, but keep held keys current.
        self.input = Input {
            jump_pressed: self.input.jump_pressed || input.jump_pressed,
            ..input
        };
        self.step(game, elapsed, pause, |session, _, _| session.take_input());
    }

    /// Lets the autopilot play. It decides afresh every tick, since a frame
    /// can hold several and the right moment to jump falls between them.
    pub fn autoplay(&mut self, game: &mut Game, elapsed: f32) {
        self.step(game, elapsed, false, |_, game, dt| {
            autopilot::input(game, dt)
        });
    }

    /// A screensaver frame, played by whoever the attract rules chose:
    /// nobody, the autopilot (unpausing a run a person left paused) or the
    /// person's keys.
    pub fn screensaver(
        &mut self,
        game: &mut Game,
        elapsed: f32,
        drive: Drive,
        input: Input,
        pause: bool,
    ) {
        match drive {
            Drive::Wait => self.advance(game, elapsed, Input::default(), false),
            Drive::Autopilot { unpause } => {
                if unpause {
                    game.toggle_pause();
                }
                self.autoplay(game, elapsed);
            }
            Drive::Person => self.advance(game, elapsed, input, pause),
        }
    }

    fn step(
        &mut self,
        game: &mut Game,
        elapsed: f32,
        pause: bool,
        mut input: impl FnMut(&mut Self, &Game, f32) -> Input,
    ) {
        let was_live = game.is_live();
        if pause {
            game.toggle_pause();
        }

        if !game.is_live() || !was_live {
            self.clock = Clock::default();
            // Idle time advances the restart delay, never the world. Starting
            // or resuming after an idle wakeup plays one tick rather than
            // fast-forwarding the run by the whole idle time.
            let dt = if matches!(game.state, State::Over(_)) {
                elapsed
            } else {
                TICK
            };
            let input = input(self, game, dt);
            game.update(dt, input);
        } else {
            for _ in 0..self.clock.advance(elapsed) {
                let input = input(self, game, TICK);
                game.update(TICK, input);
            }
        }
    }

    fn take_input(&mut self) -> Input {
        let input = self.input;
        self.input.jump_pressed = false;
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Event, RESTART_DELAY};
    use crate::rex::Pose;

    const TAP: Input = Input {
        jump_pressed: true,
        jump: false,
        duck: false,
        restart: false,
    };

    fn running() -> Game {
        let mut game = Game::new(1);
        game.state = State::Running;
        game
    }

    #[test]
    fn a_tap_on_a_frame_without_a_tick_reaches_the_next_tick() {
        let mut session = Session::default();
        let mut game = running();
        session.advance(&mut game, TICK / 2.0, TAP, false);
        assert_eq!(game.rex.pose, Pose::Running);
        session.advance(&mut game, TICK / 2.0, Input::default(), false);
        assert_eq!(game.rex.pose, Pose::Jumping);
        assert_eq!(game.events, [Event::Jump]);
    }

    #[test]
    fn a_press_is_consumed_once_across_catch_up_ticks() {
        let mut session = Session::default();
        let mut game = running();
        session.advance(&mut game, TICK * 4.0, TAP, false);
        assert_eq!(game.events, [Event::Jump]);
        for _ in 0..120 {
            session.advance(&mut game, TICK, Input::default(), false);
        }
        assert_eq!(game.rex.pose, Pose::Running);
        assert_eq!(game.events, [Event::Jump]);
    }

    #[test]
    fn starting_after_idle_does_not_advance_the_world() {
        let mut session = Session::default();
        let mut game = Game::new(1);
        session.advance(&mut game, 60.0, TAP, false);
        // The first jump plays one tick, not the whole idle time.
        assert_eq!(game.state, State::Intro(0.0));
        assert_eq!(game.rex.pose, Pose::Jumping);
        let height = game.rex.height;
        assert!(height > 0.0 && height < 10.0, "{height}");
        session.advance(&mut game, TICK, Input::default(), false);
        assert!(game.rex.height > height && game.rex.height < 2.0 * height + 1.0);
    }

    #[test]
    fn pause_discards_backlog_and_presses_while_paused() {
        let mut session = Session::default();
        let mut game = running();
        session.advance(&mut game, TICK / 2.0, Input::default(), false);
        session.advance(&mut game, TICK, TAP, true);
        session.advance(&mut game, 60.0, Input::default(), true);
        assert_eq!(game.state, State::Running);
        assert!(game.distance > 0.0 && game.distance < 4.0);
        assert_eq!(game.rex.pose, Pose::Running);
        let distance = game.distance;
        session.advance(&mut game, TICK / 2.0, Input::default(), false);
        assert_eq!(game.distance, distance);
    }

    #[test]
    fn restart_delay_uses_wall_time_while_idle() {
        let mut session = Session::default();
        let mut game = Game::new(1);
        game.state = State::Over(0.0);
        session.advance(&mut game, RESTART_DELAY / 2.0, TAP, false);
        assert!(matches!(game.state, State::Over(_)));
        session.advance(&mut game, RESTART_DELAY, TAP, false);
        assert_eq!(game.state, State::Running);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.rex.pose, Pose::Running);
    }
}
