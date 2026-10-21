use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Rusty Rex".to_owned(),
        window_width: 960,
        window_height: 320,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    loop {
        clear_background(WHITE);
        draw_line(0.0, 260.0, screen_width(), 260.0, 2.0, DARKGRAY);
        draw_text("Rusty Rex", 24.0, 48.0, 32.0, DARKGRAY);
        next_frame().await;
    }
}
