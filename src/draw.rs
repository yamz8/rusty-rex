//! Everything that puts pixels on screen. Game state is only read here.

use macroquad::prelude::*;

use crate::game::{Game, REX_X, State};
use crate::obstacle::{Kind, Obstacle};
use crate::rex::{Pose, Rex};
use crate::view;

/// Screen y of the ground line, in playfield units.
pub const GROUND_Y: f32 = 260.0;
/// The ground pebbles repeat on this spacing; each slot may hold one pebble.
const PEBBLE_SLOT: f32 = 37.0;

pub fn frame(game: &Game) {
    clear_background(WHITE);
    let area = view::visible_area(screen_width(), screen_height());
    let mut camera = Camera2D::from_display_rect(Rect::new(area.x, area.y, area.w, area.h));
    // from_display_rect flips y for render targets, which turns the scene
    // upside down when drawing straight to the window, so flip it back.
    camera.zoom.y = -camera.zoom.y;
    set_camera(&camera);

    ground(area, game.distance);
    for o in &game.obstacles {
        obstacle(o);
    }
    rex(&game.rex, game.state);

    match game.state {
        State::Waiting => centred_text("Press Space to start", 150.0, 24.0),
        State::Over(_) => {
            centred_text("G A M E   O V E R", 130.0, 32.0);
            centred_text("Press Space to play again", 165.0, 20.0);
        }
        State::Running => {}
    }
}

fn centred_text(text: &str, y: f32, size: f32) {
    let width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(
        text,
        (view::PLAYFIELD_WIDTH - width) / 2.0,
        y,
        size,
        DARKGRAY,
    );
}

/// Screen y of the top of something `height` above the ground and `h` tall.
fn top(height: f32, h: f32) -> f32 {
    GROUND_Y - height - h
}

fn obstacle(o: &Obstacle) {
    let (w, h) = o.size();
    let y = top(o.bottom, h);
    match o.kind {
        Kind::SmallCactus(n) | Kind::LargeCactus(n) => {
            let each = w / n as f32;
            for i in 0..n {
                cactus(o.x + i as f32 * each, y, each, h);
            }
        }
        Kind::Pterodactyl => pterodactyl(o.x, y, w, h, o.wings_up()),
    }
}

/// A trunk with an arm on each side, raised at different heights so a group
/// doesn't look like a fence.
fn cactus(x: f32, y: f32, w: f32, h: f32) {
    let trunk = w * 0.4;
    let trunk_x = x + (w - trunk) / 2.0;
    let arm = (w - trunk) / 2.0;
    draw_rectangle(trunk_x, y, trunk, h, DARKGRAY);
    draw_rectangle(x, y + h * 0.3, arm, h * 0.3, DARKGRAY);
    draw_rectangle(x, y + h * 0.55, trunk_x - x, 3.0, DARKGRAY);
    draw_rectangle(trunk_x + trunk, y + h * 0.2, arm, h * 0.25, DARKGRAY);
    draw_rectangle(trunk_x + trunk, y + h * 0.42, arm, 3.0, DARKGRAY);
}

/// Flies left, so the beak is on the left.
fn pterodactyl(x: f32, y: f32, w: f32, h: f32, wings_up: bool) {
    let body_y = y + h * 0.45;
    draw_rectangle(x, body_y, 14.0, 5.0, DARKGRAY);
    draw_rectangle(x + 10.0, body_y - 4.0, w - 10.0, 10.0, DARKGRAY);
    draw_rectangle(x + 14.0, body_y - 2.0, 3.0, 3.0, WHITE);
    let wing_x = x + 18.0;
    if wings_up {
        draw_triangle(
            vec2(wing_x, body_y - 4.0),
            vec2(wing_x + 16.0, body_y - 4.0),
            vec2(wing_x + 4.0, y),
            DARKGRAY,
        );
    } else {
        draw_triangle(
            vec2(wing_x, body_y + 6.0),
            vec2(wing_x + 16.0, body_y + 6.0),
            vec2(wing_x + 4.0, y + h),
            DARKGRAY,
        );
    }
}

/// Cheap integer hash, so each pebble slot keeps the same look every time it
/// scrolls into view without storing any pebbles.
fn hash(n: i64) -> u32 {
    let mut x = n as u32 ^ 0x9e37_79b9;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^ (x >> 16)
}

fn ground(area: view::Rect, distance: f32) {
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

    // Pebbles below the line are what make the ground visibly move.
    let first = ((area.x + distance) / PEBBLE_SLOT).floor() as i64 - 1;
    let last = ((area.x + area.w + distance) / PEBBLE_SLOT).ceil() as i64;
    for slot in first..=last {
        let h = hash(slot);
        if h.is_multiple_of(3) {
            continue;
        }
        let x = slot as f32 * PEBBLE_SLOT - distance + (h >> 4) as f32 % PEBBLE_SLOT;
        let y = GROUND_Y + 5.0 + ((h >> 10) % 9) as f32;
        let len = 2.0 + ((h >> 16) % 5) as f32;
        draw_rectangle(x, y, len, 1.5, DARKGRAY);
    }
}

/// Blocky stand-in for the rex sprite until real art lands.
fn rex(rex: &Rex, state: State) {
    let (w, h) = rex.size();
    let x = REX_X;
    let top = top(rex.height, h);
    let leg_h = 8.0;
    let body_h = h - leg_h;

    draw_rectangle(x, top, w, body_h, DARKGRAY);
    // A crashed rex gets Chrome's wide-eyed look.
    let eye = if matches!(state, State::Over(_)) {
        7.0
    } else {
        4.0
    };
    draw_rectangle(x + w - 12.0, top + 5.0, eye, eye, WHITE);

    // Both feet stay down mid-air, and while waiting or after a crash.
    let (back_up, front_up) = match (rex.pose, state) {
        (Pose::Jumping, _) | (_, State::Waiting | State::Over(_)) => (false, false),
        _ => (rex.back_leg_up, !rex.back_leg_up),
    };
    let legs_top = top + body_h;
    let lift = |up: bool| if up { 4.0 } else { 0.0 };
    draw_rectangle(x + 8.0, legs_top, 6.0, leg_h - lift(back_up), DARKGRAY);
    draw_rectangle(x + 22.0, legs_top, 6.0, leg_h - lift(front_up), DARKGRAY);
}
