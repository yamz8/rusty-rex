//! Plays the game by itself, for the screensaver.
//!
//! Every tick it tries running, ducking and jumping on copies of the game and
//! takes a move that survives the next stretch. The copies carry the same
//! random generator, so they even see obstacles that haven't spawned yet, and
//! the rex never crashes on a course it could clear.

use crate::game::{Game, State};
use crate::rex::Input;

/// How far ahead each move is tried. It must be longer than a full jump
/// (about 0.55 s), or a jump started too late would still look safe.
const HORIZON: f32 = 0.8;
/// After a crash the game-over screen stays up this long before the next run,
/// so it reads as a crash rather than a flicker.
pub const RESTART_PAUSE: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    /// Let go of everything: run on the ground, or cut a jump short.
    Run,
    /// Duck on the ground, or drop fast in the air.
    Duck,
    /// Take off, or keep holding for a full jump.
    Jump,
}

/// What to try on the ground once running no longer works: ducking under a
/// pterodactyl is what a person would do when both would work.
const DODGES: [Move; 2] = [Move::Duck, Move::Jump];
/// In the air, holding on for a full jump comes first, since that is what
/// the rex took off for; letting go or dropping are for tight spots.
const IN_THE_AIR: [Move; 3] = [Move::Jump, Move::Run, Move::Duck];

impl Move {
    /// The input on the tick the move starts.
    fn input(self) -> Input {
        Input {
            jump: self == Move::Jump,
            duck: self == Move::Duck,
            jump_pressed: self == Move::Jump,
        }
    }

    /// The input while the move is kept up. A press on every tick would take
    /// off again the moment the rex lands.
    fn held(self) -> Input {
        Input {
            jump_pressed: false,
            ..self.input()
        }
    }

    /// The input for this tick, pressing only from the ground.
    fn input_for(self, game: &Game) -> Input {
        if game.rex.on_ground() {
            self.input()
        } else {
            self.held()
        }
    }
}

/// The input for the next tick of `game`, which lasts `dt` seconds.
pub fn input(game: &Game, dt: f32) -> Input {
    match game.state {
        State::Running => {}
        State::Over(since) if since < RESTART_PAUSE => return Input::default(),
        // Start the first run, or the next one after a crash. Nothing was
        // held while waiting, so this is the fresh press the game asks for.
        _ => return Move::Jump.input(),
    }
    let ticks = (HORIZON / dt).ceil() as u32;
    let survives = |game: &Game, m| survives(game, m, dt, ticks);
    let chosen = if !game.rex.on_ground() {
        IN_THE_AIR.into_iter().find(|&m| survives(game, m))
    } else if survives(game, Move::Run) || can_wait(game, dt, ticks) {
        Some(Move::Run)
    } else {
        DODGES.into_iter().find(|&m| survives(game, m))
    };
    // When nothing survives, keep running and look again next tick: a jump
    // that is too early now may clear the obstacle a moment later.
    chosen.unwrap_or(Move::Run).input_for(game)
}

/// Whether the rex can run one more tick and still dodge what's coming.
/// Waiting until the last such tick times jumps the way a player does; a
/// jump started as soon as one would work clears the obstacle on its way
/// down, which looks oddly early.
fn can_wait(game: &Game, dt: f32, ticks: u32) -> bool {
    let mut next = game.clone();
    next.update(dt, Move::Run.held());
    next.state == State::Running && DODGES.into_iter().any(|m| survives(&next, m, dt, ticks))
}

/// Whether starting `m` and keeping it up for `ticks` ticks avoids a crash.
/// Holding jump after landing doesn't take off again, so a jump is one full
/// jump, then running.
fn survives(game: &Game, m: Move, dt: f32, ticks: u32) -> bool {
    let mut game = game.clone();
    let mut input = m.input_for(&game);
    for _ in 0..ticks {
        game.update(dt, input);
        input = m.held();
        if game.state != State::Running {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TICK;
    use crate::game::Event;

    fn autoplay(game: &mut Game, seconds: f32) {
        for _ in 0..(seconds / TICK).round() as u32 {
            let input = input(game, TICK);
            game.update(TICK, input);
        }
    }

    #[test]
    fn starts_by_itself() {
        let mut game = Game::new(1);
        autoplay(&mut game, 0.1);
        assert_eq!(game.state, State::Running);
    }

    #[test]
    fn survives_long_runs_past_top_speed() {
        // Five minutes reaches top speed after about two, with pterodactyls
        // in every lane and the widest cactus groups along the way.
        for seed in 1..=8 {
            let mut game = Game::new(seed);
            autoplay(&mut game, 300.0);
            assert_eq!(game.state, State::Running, "seed {seed} crashed");
        }
    }

    #[test]
    fn lands_before_jumping_again() {
        // Taking off the tick after landing means the jump before came too
        // early and fell short of the obstacle it was meant to clear.
        for seed in 1..=8 {
            let mut game = Game::new(seed);
            let mut was_on_ground = true;
            for tick in 0..(60.0 / TICK) as u32 {
                let landed = game.rex.on_ground() && !was_on_ground;
                was_on_ground = game.rex.on_ground();
                let input = input(&game, TICK);
                game.update(TICK, input);
                let jumped = game.events.drain(..).any(|e| e == Event::Jump);
                assert!(!(landed && jumped), "seed {seed} bounced at tick {tick}");
            }
        }
    }

    #[test]
    fn restarts_after_a_pause_on_the_game_over_screen() {
        let mut game = Game::new(1);
        game.state = State::Over(0.0);
        autoplay(&mut game, RESTART_PAUSE - 0.1);
        assert!(matches!(game.state, State::Over(_)));
        autoplay(&mut game, 0.2);
        assert_eq!(game.state, State::Running);
    }
}
