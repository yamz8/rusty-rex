//! Everything that puts pixels on screen. Game state is only read here.

use std::fmt::Write;

use crate::font;
use crate::game::{Game, MOON_PHASES, State};
use crate::gfx::{Color, Gfx};
use crate::obstacle::{Kind, Obstacle};
use crate::palette::{Palette, Rgb};
use crate::rex::{Pose, STANDING_SIZE};
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
/// While waiting, Chrome's rex blinks after a random wait of up to 7 s, three
/// times, then keeps its eyes open. A blink lasts one 3 fps frame.
const BLINK_WAIT: f64 = 7.0;
const BLINK_TIME: f64 = 1.0 / 3.0;
const BLINKS: i64 = 3;
/// The restart button is drawn at three times its grid's size, 54x42,
/// about Chrome's size on a canvas this wide.
const RESTART_PX: f32 = 3.0;
/// Clicks this close around the high score count as on it, as in Chrome.
const HIGH_SCORE_PADDING: f32 = 4.0;
/// Clouds drift past at this fraction of the ground's speed, so they look
/// far away; Chrome uses the same ratio.
const CLOUD_PARALLAX: f32 = 0.2;
/// Each slot this wide may hold one cloud.
const CLOUD_SLOT: f32 = 260.0;
const CLOUD_SIZE: (f32, f32) = (46.0, 14.0);
/// The banner's top edge, and the most of the sky it may cover, so it stays
/// clear of the score row's right end and the start prompt below.
const BANNER_TOP: f32 = 16.0;
const BANNER_MAX: (f32, f32) = (0.6 * view::PLAYFIELD_WIDTH, 100.0);
/// The banner is backdrop, so it is drawn well under the faint colour.
const BANNER_ALPHA: f32 = 0.35;

/// The frame's colours, converted once for drawing.
struct Colors {
    bg: Color,
    fg: Color,
    faint: Color,
    accent: Color,
}

fn color(c: Rgb) -> Color {
    Color::new(c.r, c.g, c.b, 1.0)
}

/// Score digits are this many screen pixels to a font pixel, which makes
/// them 10 wide like Chrome's.
const SCORE_PX: f32 = 2.0;
/// Font pixel sizes for headings and the lines under them.
const TITLE_PX: f32 = 3.0;
const LINE_PX: f32 = 2.0;

/// Formatting and layout are only needed when a displayed score changes.
struct ScoreText {
    value: Option<u32>,
    prefix: &'static str,
    text: String,
    art: Sprite,
}

impl ScoreText {
    fn new(prefix: &'static str) -> Self {
        Self {
            value: None,
            prefix,
            text: String::with_capacity(16),
            art: font::text(""),
        }
    }

    fn update(&mut self, value: u32) {
        if self.value == Some(value) {
            return;
        }
        // Chrome shows five digits, then six once a score passes 99999,
        // keeping the last six after that.
        let (digits, shown) = if value > 99_999 {
            (6, value % 1_000_000)
        } else {
            (5, value)
        };
        self.text.clear();
        write!(self.text, "{}{:0digits$}", self.prefix, shown).unwrap();
        self.art = font::text(&self.text);
        self.value = Some(value);
    }

    fn width(&self) -> f32 {
        self.art.cols as f32 * SCORE_PX
    }

    fn draw(&self, g: &mut Gfx, x: f32, y: f32, col: Color) {
        text(g, &self.art, x, y, SCORE_PX, col);
    }
}

/// A fixed line of text, laid out once.
struct Label {
    art: Sprite,
    px: f32,
}

impl Label {
    fn new(text: &str, px: f32) -> Self {
        Self {
            art: font::text(text),
            px,
        }
    }

    /// Draws the label centred across the playfield with its top at `y`.
    fn centred(&self, g: &mut Gfx, y: f32, col: Color) {
        let w = self.art.cols as f32 * self.px;
        text(
            g,
            &self.art,
            (view::PLAYFIELD_WIDTH - w) / 2.0,
            y,
            self.px,
            col,
        );
    }
}

/// Draws laid-out text with its top-left at (x, y).
fn text(g: &mut Gfx, art: &Sprite, x: f32, y: f32, px: f32, col: Color) {
    let (w, h) = (art.cols as f32 * px, art.rows as f32 * px);
    sprite(g, art, x, y, w, h, col, col);
}

pub struct Renderer {
    /// Where the high score was last drawn, for clicks on it.
    high_score_bounds: Option<view::Rect>,
    banner: Option<Sprite>,
    current_score: ScoreText,
    high_score: ScoreText,
    start: Label,
    muted: Label,
    paused: Label,
    resume: Label,
    game_over: Label,
    restart: Label,
}

impl Renderer {
    pub fn new(banner: Option<Sprite>) -> Self {
        Self {
            high_score_bounds: None,
            banner,
            current_score: ScoreText::new(""),
            high_score: ScoreText::new("HI "),
            start: Label::new("Press Space or click to start", LINE_PX),
            muted: Label::new("Muted", LINE_PX),
            paused: Label::new("P A U S E D", TITLE_PX),
            resume: Label::new("Press P to carry on", LINE_PX),
            game_over: Label::new("G A M E   O V E R", TITLE_PX),
            restart: Label::new("Press Space or click to play again", LINE_PX),
        }
    }

    /// Draws the game into a `width` x `height` window, `now` seconds after
    /// it opened.
    pub fn frame(
        &mut self,
        g: &mut Gfx,
        (width, height): (f32, f32),
        game: &Game,
        day: Palette,
        muted: bool,
        now: f64,
    ) {
        let p = day.at_night(game.night);
        let c = Colors {
            bg: color(p.background),
            fg: color(p.foreground),
            faint: color(p.faint),
            accent: color(p.accent),
        };

        let area = view::visible_area(width, height);
        let shown = view::shown(area);
        g.begin(c.bg, area);

        if game.night > 0.0 {
            sky(g, area, shown, game.night, game.moon_phase(), &c);
        }
        let reveal = game.reveal();
        if let Some(art) = self.banner.as_ref().filter(|_| reveal >= 1.0) {
            banner(g, art, shown, &c);
        }
        clouds(g, shown, game.distance, &c);
        ground(g, shown, game.distance, &c);
        for o in &game.obstacles {
            if shown.intersects_x(o.x, o.size().0) {
                obstacle(g, o, &c);
            }
        }
        let blink = game.state == State::Waiting && blinking(now);
        rex(g, game, blink, &c);
        self.score(g, game, &c);

        // Before the run, Chrome shows only the rex; the game then opens out
        // from it. The banner and messages stay in view throughout.
        if reveal < 1.0 {
            let edge = game.rex_x() + STANDING_SIZE.0;
            let right = shown.x + shown.w;
            let edge = edge + (right - edge) * reveal;
            g.rect(edge, area.y, area.x + area.w - edge, area.h, c.bg);
            if let Some(art) = &self.banner {
                banner(g, art, shown, &c);
            }
        }

        if muted {
            let px = self.muted.px;
            text(g, &self.muted.art, 20.0, 26.0, px, c.faint);
        }

        // Cover anything past the widest stretch the game shows, so obstacles
        // can't be seen appearing or vanishing on very wide windows.
        if shown.w < area.w {
            let right = shown.x + shown.w;
            g.rect(area.x, area.y, shown.x - area.x, area.h, c.bg);
            g.rect(right, area.y, area.x + area.w - right, area.h, c.bg);
        }

        match game.state {
            State::Waiting => self.start.centred(g, 138.0, c.fg),
            State::Paused => {
                self.paused.centred(g, 110.0, c.fg);
                self.resume.centred(g, 152.0, c.fg);
            }
            State::Over(_) => {
                self.game_over.centred(g, 80.0, c.fg);
                let icon = &sprites::sheet().restart;
                let (w, h) = (icon.cols as f32 * RESTART_PX, icon.rows as f32 * RESTART_PX);
                let x = (view::PLAYFIELD_WIDTH - w) / 2.0;
                sprite(g, icon, x, 112.0, w, h, c.fg, c.fg);
                self.restart.centred(g, 168.0, c.fg);
            }
            State::Intro(_) | State::Running => {}
        }
        g.end();
    }

    fn score(&mut self, g: &mut Gfx, game: &Game, c: &Colors) {
        let right = view::PLAYFIELD_WIDTH - 20.0;
        let y = 26.0;
        self.current_score.update(game.shown_score());
        let current = &self.current_score;
        if !game.score_hidden() {
            current.draw(g, right - current.width(), y, c.fg);
        }
        self.high_score_bounds = None;
        if game.high_score > 0 {
            self.high_score.update(game.high_score);
            let high = &self.high_score;
            let x = right - current.width() - 24.0 - high.width();
            if !game.high_score_hidden() {
                high.draw(g, x, y, c.faint);
            }
            let pad = HIGH_SCORE_PADDING;
            self.high_score_bounds = Some(view::Rect {
                x: x - pad,
                y: y - pad,
                w: high.width() + 2.0 * pad,
                h: high.art.rows as f32 * SCORE_PX + 2.0 * pad,
            });
        }
    }

    /// Whether a playfield point is on the high score as last drawn.
    pub fn on_high_score(&self, point: (f32, f32)) -> bool {
        self.high_score_bounds.is_some_and(|b| b.contains(point))
    }
}

/// The moon and stars fade in with the night, as in Chrome.
fn sky(g: &mut Gfx, area: view::Rect, shown: view::Rect, night: f32, phase: usize, c: &Colors) {
    let fade = |col: Color| Color::new(col.r, col.g, col.b, night);
    for i in 0..STAR_COUNT {
        let h = hash(i + 1000);
        let x = area.x + (h % 1000) as f32 / 1000.0 * area.w;
        let y = 20.0 + ((h >> 10) % 140) as f32;
        if shown.intersects_x(x, 2.0) {
            g.rect(x, y, 2.0, 2.0, fade(c.faint));
        }
    }
    let (moon_x, moon_y, r) = (area.x + area.w * 0.75, 70.0, 16.0);
    if !shown.intersects_x(moon_x - r, r * 2.0) {
        return;
    }
    g.circle(moon_x, moon_y, r, fade(c.fg));
    // Covering part of the full moon with the sky colour leaves the phase.
    if let Some(dx) = MOON_SHADOWS[phase] {
        g.circle(moon_x + dx, moon_y - 3.0, r, c.bg);
    }
}

/// Centres the banner in the sky at the largest whole pixel size that fits,
/// or a fraction of a pixel for art too big for whole ones.
fn banner(g: &mut Gfx, art: &Sprite, shown: view::Rect, c: &Colors) {
    let (w, h) = banner_size(art.cols, art.rows);
    let col = Color::new(c.faint.r, c.faint.g, c.faint.b, BANNER_ALPHA);
    let x = shown.x + (shown.w - w) / 2.0;
    sprite(g, art, x, BANNER_TOP, w, h, col, col);
}

/// The drawn width and height of art this many columns and rows.
fn banner_size(cols: usize, rows: usize) -> (f32, f32) {
    let fit = (BANNER_MAX.0 / cols as f32).min(BANNER_MAX.1 / rows as f32);
    let px = if fit >= 1.0 { fit.floor() } else { fit };
    (px * cols as f32, px * rows as f32)
}

/// Whether the waiting rex has its eyes shut at `t` seconds. Hashing the
/// blink number makes the waits irregular without any state.
fn blinking(t: f64) -> bool {
    let mut start = 0.0;
    for i in 0..BLINKS {
        start += f64::from(hash(i + 9000) % 1000 + 1) / 1000.0 * BLINK_WAIT;
        if (start..start + BLINK_TIME).contains(&t) {
            return true;
        }
        start += BLINK_TIME;
    }
    false
}

fn clouds(g: &mut Gfx, area: view::Rect, distance: f32, c: &Colors) {
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
            sprite(g, &sprites::sheet().cloud, x, y, w, h, c.faint, c.bg);
        }
    }
}

/// Screen y of the top of something `height` above the ground and `h` tall.
fn top(height: f32, h: f32) -> f32 {
    GROUND_Y - height - h
}

fn obstacle(g: &mut Gfx, o: &Obstacle, c: &Colors) {
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
                sprite(g, art, o.x + i as f32 * each, y, each, h, c.fg, c.bg);
            }
        }
        Kind::Pterodactyl => {
            let sheet = sprites::sheet();
            let art = if o.wings_up() {
                &sheet.pterodactyl_wings_up
            } else {
                &sheet.pterodactyl_wings_down
            };
            sprite(g, art, o.x, y, w, h, c.fg, c.bg);
        }
    }
}

/// Stretches a pixel grid over a w x h box with its top-left at (x, y).
#[allow(clippy::too_many_arguments)]
fn sprite(g: &mut Gfx, art: &Sprite, x: f32, y: f32, w: f32, h: f32, fill: Color, eye: Color) {
    let px = w / art.cols as f32;
    let py = h / art.rows as f32;
    for run in &art.runs {
        g.rect(
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

fn ground(g: &mut Gfx, area: view::Rect, distance: f32, c: &Colors) {
    // Lines are centred on their y, so shifting down by half the thickness
    // keeps the ground under the rex's feet instead of over them.
    g.rect(area.x, GROUND_Y, area.w, 2.0, c.fg);

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
            g.rect(x, y, len, 1.5, c.fg);
        }
    }
}

fn rex(g: &mut Gfx, game: &Game, blink: bool, c: &Colors) {
    let (rex, state) = (&game.rex, game.state);
    let crashed = matches!(state, State::Over(_));
    // Chrome draws the crash standing up, even when the rex was ducking.
    let (w, h) = if crashed { STANDING_SIZE } else { rex.size() };
    // Both feet stay down mid-air, and while waiting or after a crash.
    let legs = match (rex.pose, state) {
        (Pose::Jumping, _) | (_, State::Waiting | State::Over(_)) => Legs::Down,
        _ if rex.back_leg_up => Legs::BackUp,
        _ => Legs::FrontUp,
    };
    let art = sprites::sheet().rex(rex.pose == Pose::Ducking, crashed, legs);
    // Shutting the eye just paints it in the body colour.
    let eye = if blink { c.accent } else { c.bg };
    sprite(
        g,
        art,
        game.rex_x(),
        top(rex.height, h),
        w,
        h,
        c.accent,
        eye,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_waiting_rex_blinks_three_times_then_stops() {
        let step = 0.001;
        let mut blinks = 0;
        let mut shut_for = 0.0;
        let mut was_shut = false;
        for i in 0..(60.0 / step) as i64 {
            let shut = blinking(i as f64 * step);
            if shut {
                shut_for += step;
            }
            if shut && !was_shut {
                blinks += 1;
            }
            was_shut = shut;
        }
        assert_eq!(blinks, BLINKS);
        assert!((shut_for - 3.0 * BLINK_TIME).abs() < 0.02, "{shut_for}");
    }

    #[test]
    fn banners_use_whole_pixels_and_shrink_only_when_too_big() {
        // Omarchy's logo trims to 80x19: height limits it to 5 px pixels.
        assert_eq!(banner_size(80, 19), (400.0, 95.0));
        // Tiny art still stops at the height limit, in whole pixels.
        assert_eq!(banner_size(4, 2), (200.0, 100.0));
        // Art wider than the space gets pixels under one screen pixel.
        let (w, h) = banner_size(1000, 10);
        assert_eq!(w, BANNER_MAX.0);
        assert!(h < 10.0);
    }

    #[test]
    fn the_moon_is_full_once_in_its_cycle() {
        assert_eq!(MOON_SHADOWS.iter().filter(|s| s.is_none()).count(), 1);
    }

    #[test]
    fn score_text_relays_out_only_changes_and_handles_reset_and_extra_digits() {
        let mut score = ScoreText::new("");
        score.update(99_999);
        assert_eq!(score.text, "99999");
        // Five 5-pixel digits with a pixel between each.
        assert_eq!(score.width(), 29.0 * SCORE_PX);
        score.text.clear();
        score.update(99_999);
        assert_eq!(score.text, "", "unchanged score was laid out again");
        score.update(100_000);
        assert_eq!(score.text, "100000");
        assert_eq!(score.width(), 35.0 * SCORE_PX);
        score.update(0);
        assert_eq!(score.text, "00000");
        score.update(1_234_567);
        assert_eq!(score.text, "234567");

        let mut high = ScoreText::new("HI ");
        high.update(42);
        assert_eq!(high.text, "HI 00042");
    }
}
