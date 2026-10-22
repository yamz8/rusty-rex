//! Everything that puts pixels on screen. Game state is only read here.

use macroquad::prelude::*;
use std::fmt::Write;

use crate::game::{Game, MILESTONE_FLASH, MOON_PHASES, REX_X, State};
use crate::obstacle::{Kind, Obstacle};
use crate::palette::{Palette, Rgb};
use crate::rex::{Pose, Rex};
use crate::sprites::{self, Legs, Sprite};
use crate::view;

/// Screen y of the ground line, in playfield units.
pub const GROUND_Y: f32 = 260.0;
/// The ground pebbles repeat on this spacing; each slot may hold one pebble.
const PEBBLE_SLOT: f32 = 37.0;
const STAR_COUNT: i64 = 12;
/// Where the sky-coloured circle sits over the moon in each phase, from a
/// thin crescent through full (None) and back to a crescent facing the
/// other way.
const MOON_SHADOWS: [Option<f32>; MOON_PHASES] = [
    Some(8.0),
    Some(16.0),
    Some(24.0),
    None,
    Some(-24.0),
    Some(-16.0),
    Some(-8.0),
];
/// While waiting, the rex blinks once at a random moment in each slot.
const BLINK_SLOT: f64 = 4.0;
const BLINK_TIME: f64 = 0.15;
/// Clouds drift past at this fraction of the ground's speed, so they look
/// far away; Chrome uses the same ratio.
const CLOUD_PARALLAX: f32 = 0.2;
/// Each slot this wide may hold one cloud.
const CLOUD_SLOT: f32 = 260.0;
const CLOUD_SIZE: (f32, f32) = (46.0, 14.0);

/// The frame's colours, converted once for macroquad.
struct Colors {
    bg: Color,
    fg: Color,
    faint: Color,
    accent: Color,
}

fn color(c: Rgb) -> Color {
    Color::new(c.r, c.g, c.b, 1.0)
}

/// Formatting and measurement are only needed when a displayed score changes.
struct ScoreText {
    value: Option<u32>,
    prefix: &'static str,
    text: String,
    width: f32,
}

impl ScoreText {
    fn new(prefix: &'static str) -> Self {
        Self {
            value: None,
            prefix,
            text: String::with_capacity(16),
            width: 0.0,
        }
    }

    fn update(&mut self, value: u32, measure: impl FnOnce(&str) -> f32) {
        if self.value == Some(value) {
            return;
        }
        self.text.clear();
        write!(self.text, "{}{:05}", self.prefix, value).unwrap();
        self.width = measure(&self.text);
        self.value = Some(value);
    }
}

struct Label {
    text: &'static str,
    size: f32,
    width: f32,
}

impl Label {
    fn new(text: &'static str, size: f32) -> Self {
        Self {
            text,
            size,
            width: measure_text(text, None, size as u16, 1.0).width,
        }
    }

    fn centred(&self, y: f32, col: Color) {
        draw_text(
            self.text,
            (view::PLAYFIELD_WIDTH - self.width) / 2.0,
            y,
            self.size,
            col,
        );
    }
}

pub struct Renderer {
    dpi_scale: f32,
    current_score: ScoreText,
    high_score: ScoreText,
    start: Label,
    paused: Label,
    resume: Label,
    game_over: Label,
    restart: Label,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            dpi_scale: screen_dpi_scale(),
            current_score: ScoreText::new(""),
            high_score: ScoreText::new("HI "),
            start: Label::new("Press Space or click to start", 24.0),
            paused: Label::new("P A U S E D", 32.0),
            resume: Label::new("Press P to carry on", 20.0),
            game_over: Label::new("G A M E   O V E R", 32.0),
            restart: Label::new("Press Space or click to play again", 20.0),
        }
    }

    pub fn frame(&mut self, game: &Game, day: Palette, muted: bool) {
        // Font metrics can change when the window moves between monitors.
        if self.dpi_scale != screen_dpi_scale() {
            *self = Self::new();
        }
        let p = day.at_night(game.night);
        let c = Colors {
            bg: color(p.background),
            fg: color(p.foreground),
            faint: color(p.faint),
            accent: color(p.accent),
        };

        clear_background(c.bg);
        let area = view::visible_area(screen_width(), screen_height());
        let shown = view::shown(area);
        let mut camera = Camera2D::from_display_rect(Rect::new(area.x, area.y, area.w, area.h));
        // from_display_rect flips y for render targets, which turns the scene
        // upside down when drawing straight to the window, so flip it back.
        camera.zoom.y = -camera.zoom.y;
        set_camera(&camera);

        if game.night > 0.0 {
            sky(area, shown, game.night, game.moon_phase(), &c);
        }
        clouds(shown, game.distance, &c);
        ground(shown, game.distance, &c);
        for o in &game.obstacles {
            if shown.intersects_x(o.x, o.size().0) {
                obstacle(o, &c);
            }
        }
        let blink = game.state == State::Waiting && blinking(get_time());
        rex(&game.rex, game.state, blink, &c);
        self.score(game, &c);

        if muted {
            draw_text("MUTED", 20.0, 40.0, 18.0, c.faint);
        }

        // Cover anything past the widest stretch the game shows, so obstacles
        // can't be seen appearing or vanishing on very wide windows.
        if shown.w < area.w {
            let right = shown.x + shown.w;
            draw_rectangle(area.x, area.y, shown.x - area.x, area.h, c.bg);
            draw_rectangle(right, area.y, area.x + area.w - right, area.h, c.bg);
        }

        match game.state {
            State::Waiting => self.start.centred(150.0, c.fg),
            State::Paused => {
                self.paused.centred(130.0, c.fg);
                self.resume.centred(165.0, c.fg);
            }
            State::Over(_) => {
                self.game_over.centred(130.0, c.fg);
                self.restart.centred(165.0, c.fg);
            }
            State::Running => {}
        }
    }

    fn score(&mut self, game: &Game, c: &Colors) {
        // Chrome blinks the score on and off a few times at each milestone.
        let blink_off = game.milestone_flash > 0.0
            && ((MILESTONE_FLASH - game.milestone_flash) / 0.2) as u32 % 2 == 1;
        let size = 22.0;
        let right = view::PLAYFIELD_WIDTH - 20.0;
        let y = 40.0;
        let measure = |text: &str| measure_text(text, None, size as u16, 1.0).width;
        self.current_score.update(game.score(), measure);
        let current = &self.current_score;
        if !blink_off {
            draw_text(&current.text, right - current.width, y, size, c.fg);
        }
        if game.high_score > 0 {
            self.high_score.update(game.high_score, measure);
            let high = &self.high_score;
            let x = right - current.width - 24.0 - high.width;
            draw_text(&high.text, x, y, size, c.faint);
        }
    }
}

/// The moon and stars fade in with the night, as in Chrome.
fn sky(area: view::Rect, shown: view::Rect, night: f32, phase: usize, c: &Colors) {
    let fade = |col: Color| Color::new(col.r, col.g, col.b, night);
    for i in 0..STAR_COUNT {
        let h = hash(i + 1000);
        let x = area.x + (h % 1000) as f32 / 1000.0 * area.w;
        let y = 20.0 + ((h >> 10) % 140) as f32;
        if shown.intersects_x(x, 2.0) {
            draw_rectangle(x, y, 2.0, 2.0, fade(c.faint));
        }
    }
    let (moon_x, moon_y, r) = (area.x + area.w * 0.75, 70.0, 16.0);
    if !shown.intersects_x(moon_x - r, r * 2.0) {
        return;
    }
    draw_circle(moon_x, moon_y, r, fade(c.fg));
    // Covering part of the full moon with the sky colour leaves the phase.
    if let Some(dx) = MOON_SHADOWS[phase] {
        draw_circle(moon_x + dx, moon_y - 3.0, r, c.bg);
    }
}

/// Whether the waiting rex has its eyes shut at `t` seconds. Chrome's rex
/// blinks at random moments; hashing the slot keeps that irregular without
/// any state.
fn blinking(t: f64) -> bool {
    let slot = (t / BLINK_SLOT).floor();
    let at = f64::from(hash(slot as i64 + 9000) % 3000) / 1000.0;
    let into = t - slot * BLINK_SLOT;
    (at..at + BLINK_TIME).contains(&into)
}

fn clouds(area: view::Rect, distance: f32, c: &Colors) {
    let scrolled = distance * CLOUD_PARALLAX;
    let first = ((area.x + scrolled) / CLOUD_SLOT).floor() as i64 - 1;
    let last = ((area.x + area.w + scrolled) / CLOUD_SLOT).ceil() as i64;
    for slot in first..=last {
        // Offset the hash input so clouds don't line up with the pebbles.
        let seed = hash(slot + 5000);
        if seed.is_multiple_of(3) {
            continue;
        }
        let x =
            slot as f32 * CLOUD_SLOT - scrolled + (seed >> 4) as f32 % (CLOUD_SLOT - CLOUD_SIZE.0);
        let y = 70.0 + ((seed >> 12) % 80) as f32;
        let (w, h) = CLOUD_SIZE;
        if area.intersects_x(x, w) {
            sprite(&sprites::sheet().cloud, x, y, w, h, c.faint, c.bg);
        }
    }
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
            let sheet = sprites::sheet();
            let art = if matches!(o.kind, Kind::SmallCactus(_)) {
                &sheet.small_cactus
            } else {
                &sheet.large_cactus
            };
            let each = w / n as f32;
            for i in 0..n {
                sprite(art, o.x + i as f32 * each, y, each, h, c.fg, c.bg);
            }
        }
        Kind::Pterodactyl => {
            let sheet = sprites::sheet();
            let art = if o.wings_up() {
                &sheet.pterodactyl_wings_up
            } else {
                &sheet.pterodactyl_wings_down
            };
            sprite(art, o.x, y, w, h, c.fg, c.bg);
        }
    }
}

/// Stretches a pixel grid over a w x h box with its top-left at (x, y).
fn sprite(art: &Sprite, x: f32, y: f32, w: f32, h: f32, fill: Color, eye: Color) {
    let px = w / art.cols as f32;
    let py = h / art.rows as f32;
    for run in &art.runs {
        draw_rectangle(
            x + run.x as f32 * px,
            y + run.y as f32 * py,
            run.len as f32 * px,
            run.height as f32 * py,
            if run.eye { eye } else { fill },
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
        if area.intersects_x(x, len) {
            draw_rectangle(x, y, len, 1.5, c.fg);
        }
    }
}

fn rex(rex: &Rex, state: State, blink: bool, c: &Colors) {
    let (w, h) = rex.size();
    // Both feet stay down mid-air, and while waiting or after a crash.
    let legs = match (rex.pose, state) {
        (Pose::Jumping, _) | (_, State::Waiting | State::Over(_)) => Legs::Down,
        _ if rex.back_leg_up => Legs::BackUp,
        _ => Legs::FrontUp,
    };
    let art = sprites::sheet().rex(
        rex.pose == Pose::Ducking,
        matches!(state, State::Over(_)),
        legs,
    );
    // Shutting the eye just paints it in the body colour.
    let eye = if blink { c.accent } else { c.bg };
    sprite(art, REX_X, top(rex.height, h), w, h, c.accent, eye);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_waiting_rex_blinks_briefly_once_per_slot() {
        let step = 0.001;
        let mut blinks = 0;
        let mut shut_for = 0.0;
        let mut was_shut = false;
        for i in 0..(10.0 * BLINK_SLOT / step) as i64 {
            let shut = blinking(i as f64 * step);
            if shut {
                shut_for += step;
            }
            if shut && !was_shut {
                blinks += 1;
            }
            was_shut = shut;
        }
        assert_eq!(blinks, 10);
        assert!((shut_for - 10.0 * BLINK_TIME).abs() < 0.02, "{shut_for}");
    }

    #[test]
    fn the_moon_is_full_once_in_its_cycle() {
        assert_eq!(MOON_SHADOWS.iter().filter(|s| s.is_none()).count(), 1);
    }

    #[test]
    fn score_cache_measures_only_changes_and_handles_reset_and_extra_digits() {
        let mut score = ScoreText::new("");
        score.update(99_999, |s| s.len() as f32);
        assert_eq!(score.text, "99999");
        score.update(99_999, |_| panic!("unchanged score was measured again"));
        score.update(100_000, |s| s.len() as f32);
        assert_eq!(score.text, "100000");
        assert_eq!(score.width, 6.0);
        score.update(0, |s| s.len() as f32);
        assert_eq!(score.text, "00000");
        assert_eq!(score.width, 5.0);

        let mut high = ScoreText::new("HI ");
        high.update(42, |s| s.len() as f32);
        assert_eq!(high.text, "HI 00042");
        high.update(42, |_| panic!("unchanged high score was measured again"));
    }
}
