//! Everything that puts pixels on screen. Game state is only read here.

use macroquad::prelude::*;

use crate::game::{Game, REX_X};
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
    rex(&game.rex);
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
fn rex(rex: &Rex) {
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
