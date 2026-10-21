mod clock;
mod draw;
mod game;
mod obstacle;
mod rex;
mod storage;
mod view;

use clock::{Clock, TICK};
use game::Game;
use macroquad::prelude::*;
use rex::Input;
use view::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

fn window_conf() -> Conf {
    Conf {
        window_title: "Rusty Rex".to_owned(),
        window_width: PLAYFIELD_WIDTH as i32,
        window_height: PLAYFIELD_HEIGHT as i32,
        ..Default::default()
    }
}

fn read_input() -> Input {
    Input {
        jump: is_key_down(KeyCode::Space) || is_key_down(KeyCode::Up),
        duck: is_key_down(KeyCode::Down),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Any changing seed will do; it only decides the order of obstacles.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    let mut game = Game::new(seed);
    let score_path = storage::default_high_score_path();
    if let Some(path) = &score_path {
        game.high_score = storage::load(path);
    }
    let mut saved_high_score = game.high_score;
    let mut clock = Clock::default();
    loop {
        let input = read_input();
        for _ in 0..clock.advance(get_frame_time()) {
            game.update(TICK, input);
        }
        if game.high_score > saved_high_score {
            saved_high_score = game.high_score;
            // Losing a high score isn't worth crashing the game over.
            if let Some(path) = &score_path
                && let Err(e) = storage::save(path, saved_high_score)
            {
                eprintln!(
                    "rusty-rex: couldn't save high score to {}: {e}",
                    path.display()
                );
            }
        }
        draw::frame(&game);
        next_frame().await;
    }
}
