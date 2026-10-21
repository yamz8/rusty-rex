mod clock;
mod draw;
mod game;
mod obstacle;
mod rex;
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
    let mut clock = Clock::default();
    loop {
        let input = read_input();
        for _ in 0..clock.advance(get_frame_time()) {
            game.update(TICK, input);
        }
        draw::frame(&game);
        next_frame().await;
    }
}
