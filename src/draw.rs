//! Everything that puts pixels on screen. Game state is only read here.

use macroquad::prelude::*;

use crate::game::{Game, MILESTONE_FLASH, REX_X, State};
use crate::obstacle::{Kind, Obstacle};
use crate::palette::{Palette, Rgb};
use crate::rex::{Pose, Rex};
use crate::view;

/// Screen y of the ground line, in playfield units.
pub const GROUND_Y: f32 = 260.0;
/// The ground pebbles repeat on this spacing; each slot may hold one pebble.
const PEBBLE_SLOT: f32 = 37.0;
const STAR_COUNT: i64 = 12;

/// The frame's colours, converted once for macroquad.
struct Colors {
    bg: Color,
    fg: Color,
    faint: Color,
}

fn color(c: Rgb) -> Color {
    Color::new(c.r, c.g, c.b, 1.0)
}

pub fn frame(game: &Game, day: Palette) {
    let p = day.at_night(game.night);
    let c = Colors {
        bg: color(p.background),
        fg: color(p.foreground),
        faint: color(p.faint),
    };

    clear_background(c.bg);
    let area = view::visible_area(screen_width(), screen_height());
    let mut camera = Camera2D::from_display_rect(Rect::new(area.x, area.y, area.w, area.h));
    // from_display_rect flips y for render targets, which turns the scene
    // upside down when drawing straight to the window, so flip it back.
    camera.zoom.y = -camera.zoom.y;
    set_camera(&camera);

    if game.night > 0.0 {
        sky(area, game.night, &c);
    }
    ground(area, game.distance, &c);
    for o in &game.obstacles {
        obstacle(o, &c);
    }
    rex(&game.rex, game.state, &c);
    score(game, &c);

    match game.state {
        State::Waiting => centred_text("Press Space to start", 150.0, 24.0, c.fg),
        State::Over(_) => {
            centred_text("G A M E   O V E R", 130.0, 32.0, c.fg);
            centred_text("Press Space to play again", 165.0, 20.0, c.fg);
        }
        State::Running => {}
    }
}

/// The moon and stars fade in with the night, as in Chrome.
fn sky(area: view::Rect, night: f32, c: &Colors) {
    let fade = |col: Color| Color::new(col.r, col.g, col.b, night);
    for i in 0..STAR_COUNT {
        let h = hash(i + 1000);
        let x = area.x + (h % 1000) as f32 / 1000.0 * area.w;
        let y = 20.0 + ((h >> 10) % 140) as f32;
        draw_rectangle(x, y, 2.0, 2.0, fade(c.faint));
    }
    let (moon_x, moon_y, r) = (area.x + area.w * 0.75, 70.0, 16.0);
    draw_circle(moon_x, moon_y, r, fade(c.fg));
    // Covering part of the full moon with the sky colour leaves a crescent.
    draw_circle(moon_x + 8.0, moon_y - 3.0, r, c.bg);
}

fn score(game: &Game, c: &Colors) {
    // Chrome blinks the score on and off a few times at each milestone.
    let blink_off = game.milestone_flash > 0.0
        && ((MILESTONE_FLASH - game.milestone_flash) / 0.2) as u32 % 2 == 1;
    let size = 22.0;
    let right = view::PLAYFIELD_WIDTH - 20.0;
    let y = 40.0;
    let current = format!("{:05}", game.score());
    let current_width = measure_text(&current, None, size as u16, 1.0).width;
    if !blink_off {
        draw_text(&current, right - current_width, y, size, c.fg);
    }
    if game.high_score > 0 {
        let high = format!("HI {:05}", game.high_score);
        let high_width = measure_text(&high, None, size as u16, 1.0).width;
        let x = right - current_width - 24.0 - high_width;
        draw_text(&high, x, y, size, c.faint);
    }
}

fn centred_text(text: &str, y: f32, size: f32, col: Color) {
    let width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, (view::PLAYFIELD_WIDTH - width) / 2.0, y, size, col);
}

/// Screen y of the top of something `height` above the ground and `h` tall.
fn top(height: f32, h: f32) -> f32 {
    GROUND_Y - height - h
}

fn obstacle(o: &Obstacle, c: &Colors) {
    let (w, h) = o.size();
    let y = top(o.bottom, h);
    match o.kind {
        Kind::SmallCactus(n) | Kind::LargeCactus(n) => {
            let each = w / n as f32;
            for i in 0..n {
                cactus(o.x + i as f32 * each, y, each, h, c);
            }
        }
        Kind::Pterodactyl => pterodactyl(o.x, y, w, h, o.wings_up(), c),
    }
}

/// A trunk with an arm on each side, raised at different heights so a group
/// doesn't look like a fence.
fn cactus(x: f32, y: f32, w: f32, h: f32, c: &Colors) {
    let trunk = w * 0.4;
    let trunk_x = x + (w - trunk) / 2.0;
    let arm = (w - trunk) / 2.0;
    draw_rectangle(trunk_x, y, trunk, h, c.fg);
    draw_rectangle(x, y + h * 0.3, arm, h * 0.3, c.fg);
    draw_rectangle(x, y + h * 0.55, trunk_x - x, 3.0, c.fg);
    draw_rectangle(trunk_x + trunk, y + h * 0.2, arm, h * 0.25, c.fg);
    draw_rectangle(trunk_x + trunk, y + h * 0.42, arm, 3.0, c.fg);
}

/// Flies left, so the beak is on the left.
fn pterodactyl(x: f32, y: f32, w: f32, h: f32, wings_up: bool, c: &Colors) {
    let body_y = y + h * 0.45;
    draw_rectangle(x, body_y, 14.0, 5.0, c.fg);
    draw_rectangle(x + 10.0, body_y - 4.0, w - 10.0, 10.0, c.fg);
    draw_rectangle(x + 14.0, body_y - 2.0, 3.0, 3.0, c.bg);
    let wing_x = x + 18.0;
    if wings_up {
        draw_triangle(
            vec2(wing_x, body_y - 4.0),
            vec2(wing_x + 16.0, body_y - 4.0),
            vec2(wing_x + 4.0, y),
            c.fg,
        );
    } else {
        draw_triangle(
            vec2(wing_x, body_y + 6.0),
            vec2(wing_x + 16.0, body_y + 6.0),
            vec2(wing_x + 4.0, y + h),
            c.fg,
        );
    }
}

/// Cheap integer hash, so each pebble slot and star keeps the same look every
/// time it is drawn without storing any of them.
fn hash(n: i64) -> u32 {
    let mut x = n as u32 ^ 0x9e37_79b9;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^ (x >> 16)
}

fn ground(area: view::Rect, distance: f32, c: &Colors) {
    // Lines are centred on their y, so shifting down by half the thickness
    // keeps the ground under the rex's feet instead of over them.
    draw_line(
        area.x,
        GROUND_Y + 1.0,
        area.x + area.w,
        GROUND_Y + 1.0,
        2.0,
        c.fg,
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
        draw_rectangle(x, y, len, 1.5, c.fg);
    }
}

/// Blocky stand-in for the rex sprite until real art lands.
fn rex(rex: &Rex, state: State, c: &Colors) {
    let (w, h) = rex.size();
    let x = REX_X;
    let top = top(rex.height, h);
    let leg_h = 8.0;
    let body_h = h - leg_h;

    draw_rectangle(x, top, w, body_h, c.fg);
    // A crashed rex gets Chrome's wide-eyed look.
    let eye = if matches!(state, State::Over(_)) {
        7.0
    } else {
        4.0
    };
    draw_rectangle(x + w - 12.0, top + 5.0, eye, eye, c.bg);

    // Both feet stay down mid-air, and while waiting or after a crash.
    let (back_up, front_up) = match (rex.pose, state) {
        (Pose::Jumping, _) | (_, State::Waiting | State::Over(_)) => (false, false),
        _ => (rex.back_leg_up, !rex.back_leg_up),
    };
    let legs_top = top + body_h;
    let lift = |up: bool| if up { 4.0 } else { 0.0 };
    draw_rectangle(x + 8.0, legs_top, 6.0, leg_h - lift(back_up), c.fg);
    draw_rectangle(x + 22.0, legs_top, 6.0, leg_h - lift(front_up), c.fg);
}
