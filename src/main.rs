mod clock;
mod rex;
mod view;

use clock::{Clock, TICK};
use macroquad::prelude::*;
use rex::{Input, Pose, Rex};
use view::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

const GROUND_Y: f32 = 260.0;
const REX_X: f32 = 60.0;

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

/// Blocky stand-in for the rex sprite until real art lands.
fn draw_rex(rex: &Rex) {
    let (w, h) = rex.size();
    let x = REX_X;
    let top = GROUND_Y - rex.height - h;
    let leg_h = 8.0;
    let body_h = h - leg_h;

    draw_rectangle(x, top, w, body_h, DARKGRAY);
    draw_rectangle(x + w - 12.0, top + 5.0, 4.0, 4.0, WHITE);

    let (back_up, front_up) = match rex.pose {
        Pose::Jumping => (false, false),
        _ => (rex.back_leg_up, !rex.back_leg_up),
    };
    let legs_top = top + body_h;
    let lift = |up: bool| if up { 4.0 } else { 0.0 };
    draw_rectangle(x + 8.0, legs_top, 6.0, leg_h - lift(back_up), DARKGRAY);
    draw_rectangle(x + 22.0, legs_top, 6.0, leg_h - lift(front_up), DARKGRAY);
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rex = Rex::new();
    let mut clock = Clock::default();
    loop {
        let input = read_input();
        for _ in 0..clock.advance(get_frame_time()) {
            rex.update(TICK, input);
        }

        clear_background(WHITE);
        let area = view::visible_area(screen_width(), screen_height());
        let mut camera = Camera2D::from_display_rect(Rect::new(area.x, area.y, area.w, area.h));
        // from_display_rect flips y for render targets, which turns the scene
        // upside down when drawing straight to the window, so flip it back.
        camera.zoom.y = -camera.zoom.y;
        set_camera(&camera);
        // Lines are centred on their y, so shifting down by half the thickness
        // keeps the ground under the rex's feet instead of over them.
        draw_line(
            area.x,
            GROUND_Y + 1.0,
            area.x + area.w,
            GROUND_Y + 1.0,
            2.0,
            DARKGRAY,
        );
        draw_rex(&rex);
        next_frame().await;
    }
}
