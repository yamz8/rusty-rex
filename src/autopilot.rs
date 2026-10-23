//! Plays the game by itself, for the screensaver.
//!
//! Every tick it tries running, ducking and jumping on copies of the game and
//! takes a move that survives the next stretch. The copies carry the same
//! random generator, so they even see obstacles that haven't spawned yet, and
//! the rex never crashes on a course it could clear.

use crate::game::{Game, REX_X, State};
use crate::rex::{Input, MIN_JUMP_HEIGHT};

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
    /// Take off and let go as soon as Chrome lets that cut the jump short,
    /// for the quickest hop: about 0.4 s against a full jump's 0.55.
    Hop,
}

/// What to try on the ground once running no longer works: ducking under a
/// pterodactyl is what a person would do when both would work, and a full
/// jump over a short hop.
const DODGES: [Move; 3] = [Move::Duck, Move::Jump, Move::Hop];
/// Once a jump lands or an obstacle goes by, any move may come next.
const NEXT: [Move; 4] = [Move::Run, Move::Duck, Move::Jump, Move::Hop];
/// In the air, holding on for a full jump comes first, since that is what
/// the rex took off for; letting go or dropping are for tight spots.
const IN_THE_AIR: [Move; 3] = [Move::Jump, Move::Run, Move::Duck];

impl Move {
    /// The input on the tick the move starts.
    fn input(self) -> Input {
        let takes_off = matches!(self, Move::Jump | Move::Hop);
        Input {
            jump: takes_off,
            duck: self == Move::Duck,
            jump_pressed: takes_off,
            restart: false,
        }
    }

    /// The input while the move is kept up. A press on every tick would take
    /// off again the moment the rex lands.
    fn held(self, game: &Game) -> Input {
        let hopping = self == Move::Hop && game.rex.height <= MIN_JUMP_HEIGHT;
        Input {
            jump: self == Move::Jump || hopping,
            jump_pressed: false,
            ..self.input()
        }
    }

    /// The input for this tick, pressing only from the ground.
    fn input_for(self, game: &Game) -> Input {
        if game.rex.on_ground() {
            self.input()
        } else {
            self.held(game)
        }
    }
}

/// The input for the next tick of `game`, which lasts `dt` seconds.
pub fn input(game: &Game, dt: f32) -> Input {
    match game.state {
        State::Running => {}
        // Nothing comes during the intro; let the first jump play out.
        State::Intro(_) => return Input::default(),
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
    next.update(dt, Move::Run.held(game));
    next.state == State::Running && DODGES.into_iter().any(|m| survives(&next, m, dt, ticks))
}

/// Whether starting `m` and keeping it up for `ticks` ticks avoids a crash.
/// Chrome's gaps can be shorter than the look-ahead, so once a jump lands
/// or an obstacle goes by, whichever move gets past the next one follows.
fn survives(game: &Game, m: Move, dt: f32, ticks: u32) -> bool {
    // Nothing in reach means no crash, so there is nothing to simulate.
    // Reach is asked afresh at each landing or pass, and every tick.
    if game.clear_for(ticks as f32 * dt) {
        return true;
    }
    let mut game = game.clone();
    let mut input = m.input_for(&game);
    for tick in 1..=ticks {
        let airborne = !game.rex.on_ground();
        let ahead = obstacles_ahead(&game);
        game.update(dt, input);
        input = m.held(&game);
        if game.state != State::Running {
            return false;
        }
        let landed = airborne && game.rex.on_ground();
        let passed = game.rex.on_ground() && obstacles_ahead(&game) < ahead;
        if landed || passed {
            let rest = ticks - tick;
            return rest == 0 || NEXT.into_iter().any(|next| survives(&game, next, dt, rest));
        }
    }
    true
}

/// Obstacles the rex has yet to get past.
fn obstacles_ahead(game: &Game) -> usize {
    game.obstacles.iter().filter(|o| o.right() > REX_X).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rex::STANDING_SIZE;

    /// A full jump lasts about this long.
    const FULL_JUMP: f32 = 0.6;
    use crate::clock::TICK;
    use crate::game::{Event, MAX_SPEED, START_SPEED};

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
        assert!(matches!(game.state, State::Intro(_)));
        autoplay(&mut game, 1.5);
        assert_eq!(game.state, State::Running);
    }

    #[test]
    fn survives_long_runs_past_top_speed() {
        // Five minutes reaches top speed after about two, with pterodactyls
        // in every lane and the widest cactus groups along the way. Seeds
        // far apart, since nearby ones once hid a planner that couldn't
        // follow one jump with another.
        for seed in (1..=8).map(|s| s * 7919) {
            let mut game = Game::new(seed);
            autoplay(&mut game, 300.0);
            assert_eq!(game.state, State::Running, "seed {seed} crashed");
        }
    }

    #[test]
    fn survives_long_runs_as_the_omarchy_runner() {
        // The Omarchy runner has its own size and boxes, so the autopilot is
        // held to the same five minutes with them.
        for seed in (1..=4).map(|s| s * 7919) {
            let mut game = Game::new(seed);
            game.runner = crate::runner::Runner::Omarchy;
            autoplay(&mut game, 300.0);
            assert_eq!(game.state, State::Running, "seed {seed} crashed");
        }
    }

    #[test]
    fn jumps_again_on_landing_only_when_it_must() {
        // Chrome's gaps can be shorter than a jump, so taking off the tick
        // after landing is sometimes the only way through. When the next
        // obstacle is further than a full jump, it means the jump before
        // came too early.
        for seed in 1..=8 {
            let mut game = Game::new(seed);
            let mut was_on_ground = true;
            for tick in 0..(60.0 / TICK) as u32 {
                let landed = game.rex.on_ground() && !was_on_ground;
                was_on_ground = game.rex.on_ground();
                let input = input(&game, TICK);
                let before = game.clone();
                game.update(TICK, input);
                let jumped = game.events.drain(..).any(|e| e == Event::Jump);
                if landed && jumped {
                    let next = before.obstacles.iter().find(|o| o.right() > REX_X);
                    let room = next.map_or(f32::INFINITY, |o| o.x - REX_X - STANDING_SIZE.0);
                    assert!(
                        room < before.speed * FULL_JUMP,
                        "seed {seed} bounced at tick {tick} with {room} px to spare"
                    );
                }
            }
        }
    }

    #[test]
    fn a_clear_course_survives_any_input() {
        // The look-ahead trusts clear_for to skip simulating, so whenever it
        // holds, no way of playing may crash in that time: not even jumping
        // or ducking into an obstacle on purpose. Starting some runs at top
        // speed tries the tightest margin, with pterodactyls flying fast.
        let mut clear = 0;
        for runner in [crate::runner::Runner::Rex, crate::runner::Runner::Omarchy] {
            for (seed, speed) in (1..=4).flat_map(|s| [(s, START_SPEED), (s, MAX_SPEED)]) {
                let mut game = Game::new(seed);
                game.runner = runner;
                game.speed = speed;
                for tick in 0..(120.0 / TICK) as u32 {
                    if tick % 4 == 0 && game.state == State::Running && game.clear_for(HORIZON) {
                        clear += 1;
                        for m in NEXT {
                            let mut copy = game.clone();
                            for _ in 0..(HORIZON / TICK).ceil() as u32 {
                                copy.update(TICK, m.input_for(&copy));
                            }
                            assert_eq!(
                                copy.state,
                                State::Running,
                                "{runner:?} {seed} {speed} {m:?}"
                            );
                        }
                    }
                    let input = input(&game, TICK);
                    game.update(TICK, input);
                }
            }
        }
        assert!(clear > 1000, "only {clear} clear moments tried");
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
