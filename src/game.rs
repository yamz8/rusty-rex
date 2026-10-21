//! The running game: the world scrolling past the rex, getting faster.
//!
//! Like the rex, this never touches macroquad. Horizontal positions are in
//! playfield pixels and heights in pixels above the ground.

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

#[derive(Debug)]
pub struct Game {
    pub rex: Rex,
    /// Scroll speed in pixels per second.
    pub speed: f32,
    /// How far the world has scrolled in total.
    pub distance: f32,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            rex: Rex::new(),
            speed: START_SPEED,
            distance: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: Input) {
        self.rex.update(dt, input);
        self.distance += self.speed * dt;
        self.speed = (self.speed + ACCELERATION * dt).min(MAX_SPEED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn run(game: &mut Game, seconds: f32) {
        for _ in 0..(seconds / DT).round() as u32 {
            game.update(DT, Input::default());
        }
    }

    #[test]
    fn starts_at_chrome_speed() {
        let game = Game::new();
        assert_eq!(game.speed, START_SPEED);
        assert_eq!(game.distance, 0.0);
    }

    #[test]
    fn scrolls_by_speed_times_time() {
        let mut game = Game::new();
        run(&mut game, 1.0);
        // The speed creeps up during the second, so allow a little over.
        assert!(game.distance >= START_SPEED);
        assert!(game.distance < START_SPEED + ACCELERATION);
    }

    #[test]
    fn speeds_up_over_time() {
        let mut game = Game::new();
        run(&mut game, 10.0);
        let expected = START_SPEED + 10.0 * ACCELERATION;
        assert!((game.speed - expected).abs() < 0.1, "speed {}", game.speed);
    }

    #[test]
    fn speed_is_capped() {
        let mut game = Game::new();
        run(&mut game, 200.0);
        assert_eq!(game.speed, MAX_SPEED);
    }
}
