//! Bridges window frames and input events to fixed physics ticks.

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
        let was_running = game.state == State::Running;
        if pause {
            game.toggle_pause();
        }

        if game.state != State::Running || !was_running {
            self.clock = Clock::default();
            // Idle time advances the restart delay, never the world. Starting
            // or resuming after an idle wakeup must not fast-forward a run.
            let dt = if matches!(game.state, State::Over(_)) {
                elapsed
            } else {
                0.0
            };
            let input = self.take_input();
            game.update(dt, input);
        } else {
            for _ in 0..self.clock.advance(elapsed) {
                let input = self.take_input();
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
        assert_eq!(game.state, State::Running);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.rex.pose, Pose::Jumping);
        session.advance(&mut game, TICK, Input::default(), false);
        assert!(game.distance > 0.0 && game.distance < 4.0);
    }

    #[test]
    fn pause_discards_backlog_and_presses_while_paused() {
        let mut session = Session::default();
        let mut game = running();
        session.advance(&mut game, TICK / 2.0, Input::default(), false);
        session.advance(&mut game, TICK, TAP, true);
        session.advance(&mut game, 60.0, Input::default(), true);
        assert_eq!(game.state, State::Running);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.rex.pose, Pose::Running);
        session.advance(&mut game, TICK / 2.0, Input::default(), false);
        assert_eq!(game.distance, 0.0);
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
