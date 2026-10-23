//! Everything that puts pixels on screen. Game state is only read here.

use std::fmt::Write;

use crate::font;
use crate::game::{Game, MOON_PHASES, State};
use crate::gfx::{Color, Gfx};
use crate::obstacle::{Kind, Obstacle};
use crate::palette::{Palette, Rgb};
use crate::rex::{Pose, STANDING_SIZE};
use crate::runner::Runner;
use crate::sprites::{self, Legs, Sprite};
use crate::view;

/// Screen y of the ground line, in playfield units.
pub const GROUND_Y: f32 = 260.0;
/// The ground pebbles repeat on this spacing; each slot may hold one pebble.
const PEBBLE_SLOT: f32 = 37.0;
/// Chrome's night has two stars and a moon, drifting left at 0.3 and 0.25
/// px/frame.
const STAR_COUNT: i64 = 2;
const STAR_SPEED: f32 = 18.0;
const MOON_SPEED: f32 = 15.0;
/// The ground comes in stretches this long, each flat or bumpy, as in
/// Chrome's two-part horizon line.
const GROUND_STRETCH: f32 = 600.0;
/// In a bumpy stretch, each slot this wide may hold a bump.
const BUMP_SLOT: f32 = 50.0;
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
/// The restart button is drawn at twice its grid's size: Chrome's 36x32.
const RESTART_PX: f32 = 2.0;
/// Clicks this close around the high score count as on it, as in Chrome.
const HIGH_SCORE_PADDING: f32 = 4.0;
/// Each slot this wide may hold one cloud.
const CLOUD_SLOT: f32 = 260.0;
const CLOUD_SIZE: (f32, f32) = (46.0, 14.0);
/// The banner's top edge, and the most of the sky it may cover: a band at
/// the top, clear of the score row's right end, with the clouds, moon and
/// stars kept below it.
const BANNER_TOP: f32 = 12.0;
const BANNER_MAX: (f32, f32) = (0.6 * view::PLAYFIELD_WIDTH, 76.0);
/// Room left between the banner and what drifts below it.
const BANNER_GAP: f32 = 8.0;
/// Where the clouds, moon and stars sit without a banner: clouds from 70 to
/// 150, the moon at 70, stars from 20 to 120.
const CLOUD_BAND: (f32, f32) = (70.0, 150.0);
const STAR_BAND: (f32, f32) = (20.0, 120.0);
const MOON_Y: f32 = 70.0;
const MOON_RADIUS: f32 = 16.0;
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
}

impl Renderer {
    /// `screensaver` says how to start: a person can play it, or leave.
    pub fn new(banner: Option<Sprite>, screensaver: bool) -> Self {
        let start = if screensaver {
            "Space to play   Esc to leave"
        } else {
            "Press Space or click to start"
        };
        Self {
            high_score_bounds: None,
            banner,
            current_score: ScoreText::new(""),
            high_score: ScoreText::new("HI "),
            start: Label::new(start, LINE_PX),
            muted: Label::new("Muted", LINE_PX),
            paused: Label::new("P A U S E D", TITLE_PX),
            resume: Label::new("Press P to carry on", LINE_PX),
            game_over: Label::new("G A M E   O V E R", TITLE_PX),
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

        // Nothing that drifts goes above this: the bottom of the banner.
        let floor = self.banner.as_ref().map_or(0.0, |art| {
            BANNER_TOP + banner_size(art.cols, art.rows).1 + BANNER_GAP
        });
        if game.night > 0.0 {
            sky(g, shown, game, floor, &c);
        }
        let reveal = game.reveal();
        if let Some(art) = self.banner.as_ref().filter(|_| reveal >= 1.0) {
            banner(g, art, shown, &c);
        }
        clouds(g, shown, game.clouds, floor, &c);
        ground(g, shown, game.distance, &c);
        for o in &game.obstacles {
            if shown.intersects_x(o.x, o.size().0) {
                obstacle(g, o, &c);
            }
        }
        let blink = game.state == State::Waiting && blinking(now);
        match game.runner {
            Runner::Rex => rex(g, game, blink, &c),
            Runner::Omarchy => omarchy(g, game, blink, &c),
        }
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
            // As in Chrome, the button is the only hint how to play again.
            State::Over(_) => {
                self.game_over.centred(g, 96.0, c.fg);
                let icon = &sprites::sheet().restart;
                let (w, h) = (icon.cols as f32 * RESTART_PX, icon.rows as f32 * RESTART_PX);
                let x = (view::PLAYFIELD_WIDTH - w) / 2.0;
                sprite(g, icon, x, 132.0, w, h, c.fg, c.fg);
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

/// The moon and stars fade in with the night and drift left while it lasts,
/// wrapping round, and the stars move to new places each night, as in
/// Chrome. They stay below `floor`.
fn sky(g: &mut Gfx, shown: view::Rect, game: &Game, floor: f32, c: &Colors) {
    let fade = |col: Color| Color::new(col.r, col.g, col.b, game.night);
    // Wraps a drifting x so it re-enters from the right of the shown area.
    let drift = |start: f32, speed: f32, w: f32| {
        let span = shown.w + 2.0 * w;
        shown.x - w + (start - speed * game.sky_time).rem_euclid(span)
    };
    let star = &sprites::sheet().star;
    for i in 0..STAR_COUNT {
        let h = hash(i64::from(game.moon_phase() as u32) * STAR_COUNT + i + 1000);
        let start = (h % 1000) as f32 / 1000.0 * shown.w;
        let x = drift(start, STAR_SPEED, 9.0);
        let y = in_band(STAR_BAND, floor, h >> 10);
        sprite(g, star, x, y, 9.0, 9.0, fade(c.faint), fade(c.faint));
    }
    let r = MOON_RADIUS;
    let (moon_x, moon_y) = (
        drift(shown.w * 0.75, MOON_SPEED, 2.0 * r) + r,
        moon_y(floor),
    );
    g.circle(moon_x, moon_y, r, fade(c.fg));
    // Covering part of the full moon with the sky colour leaves the phase.
    if let Some(dx) = MOON_SHADOWS[game.moon_phase()] {
        g.circle(moon_x + dx, moon_y - 3.0, r, c.bg);
    }
}

/// Centres the banner in the sky at the largest size in half-pixel steps
/// that fits, or a fraction of a pixel for art too big for those.
fn banner(g: &mut Gfx, art: &Sprite, shown: view::Rect, c: &Colors) {
    let (w, h) = banner_size(art.cols, art.rows);
    let col = Color::new(c.faint.r, c.faint.g, c.faint.b, BANNER_ALPHA);
    let x = shown.x + (shown.w - w) / 2.0;
    sprite(g, art, x, BANNER_TOP, w, h, col, col);
}

/// The drawn width and height of art this many columns and rows.
fn banner_size(cols: usize, rows: usize) -> (f32, f32) {
    let fit = (BANNER_MAX.0 / cols as f32).min(BANNER_MAX.1 / rows as f32);
    // Half-pixel steps: a half block is two of the art's pixels, so this
    // keeps half-block art at the size whole steps used to give it.
    let px = if fit >= 1.0 {
        (fit * 2.0).floor() / 2.0
    } else {
        fit
    };
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

/// A height in `band` picked by `hash`, moved down below `floor` if the band
/// starts above it; the band keeps its bottom, so it only gets narrower.
fn in_band((top, bottom): (f32, f32), floor: f32, hash: u32) -> f32 {
    let top = top.max(floor);
    let span = (bottom - top).max(1.0) as u32;
    top + (hash % span) as f32
}

/// The moon's centre: its usual height, or just under the floor.
fn moon_y(floor: f32) -> f32 {
    MOON_Y.max(floor + MOON_RADIUS)
}

fn clouds(g: &mut Gfx, area: view::Rect, scrolled: f32, floor: f32, c: &Colors) {
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
        let y = in_band(CLOUD_BAND, floor, seed >> 12);
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

    // Bumps in the line, in stretches that are bumpy or flat at random.
    let first = ((area.x + distance) / BUMP_SLOT).floor() as i64 - 1;
    let last = ((area.x + area.w + distance) / BUMP_SLOT).ceil() as i64;
    for slot in first..=last {
        let stretch = (slot as f32 * BUMP_SLOT / GROUND_STRETCH).floor() as i64;
        let h = hash(slot + 3000);
        if hash(stretch + 7000).is_multiple_of(2) || h.is_multiple_of(3) {
            continue;
        }
        let x = slot as f32 * BUMP_SLOT - distance + (h >> 4) as f32 % (BUMP_SLOT - 8.0);
        if area.intersects_x(x, 8.0) {
            g.rect(x, GROUND_Y - 1.0, 8.0, 1.0, c.fg);
            g.rect(x + 2.0, GROUND_Y - 2.0, 4.0, 1.0, c.fg);
        }
    }

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

/// Which of the Omarchy runner's sprites shows its pose, in the order of
/// the sheet's `omarchy`.
fn omarchy_pose(game: &Game) -> usize {
    match (game.state, game.rex.pose) {
        (State::Over(_), _) => 4,
        (_, Pose::Ducking) => 3,
        // both feet stay down mid-air and while waiting, as the rex's do
        (_, Pose::Jumping) | (State::Waiting, _) => 2,
        _ if game.rex.back_leg_up => 1,
        _ => 0,
    }
}

/// The Omarchy runner, drawn as the rex is: its sprite stretched over its
/// box, in the accent. Its eyes are drawn on its hollow face, so a blink
/// paints them in the background colour.
fn omarchy(g: &mut Gfx, game: &Game, blink: bool, c: &Colors) {
    let crashed = matches!(game.state, State::Over(_));
    // a crash while ducking is drawn standing up, as the rex's is
    let pose = if crashed {
        Pose::Running
    } else {
        game.rex.pose
    };
    let (w, h) = Runner::Omarchy.size(pose);
    let art = &sprites::sheet().omarchy[omarchy_pose(game)];
    let eye = if blink { c.bg } else { c.accent };
    sprite(
        g,
        art,
        game.rex_x(),
        top(game.rex.height, h),
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
    fn the_omarchy_runner_has_a_sprite_for_every_pose() {
        let mut game = Game::new(1);
        game.runner = Runner::Omarchy;
        let mut seen = vec![];
        for (state, pose, back) in [
            (State::Running, Pose::Running, false),
            (State::Running, Pose::Running, true),
            (State::Running, Pose::Jumping, true),
            (State::Running, Pose::Ducking, true),
            (State::Over(0.5), Pose::Ducking, true),
        ] {
            game.state = state;
            game.rex.pose = pose;
            game.rex.back_leg_up = back;
            seen.push(omarchy_pose(&game));
        }
        assert_eq!(seen, [0, 1, 2, 3, 4]);
        // each sprite's grid has the shape of the box it is stretched over
        let sheet = sprites::sheet();
        for (i, pose) in [
            Pose::Running,
            Pose::Running,
            Pose::Jumping,
            Pose::Ducking,
            Pose::Running,
        ]
        .into_iter()
        .enumerate()
        {
            let (w, h) = Runner::Omarchy.size(pose);
            let art = &sheet.omarchy[i];
            let cell = crate::runner::OMARCHY_CELL;
            assert_eq!(
                (art.cols as f32 * cell, art.rows as f32 * cell),
                (w, h),
                "pose {i}"
            );
        }
    }

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
    fn banners_use_half_pixel_steps_and_shrink_only_when_too_big() {
        // Omarchy's logo trims to 160x38: height limits it to 2 px pixels.
        assert_eq!(banner_size(160, 38), (320.0, 76.0));
        // Tiny art still stops at the height limit.
        assert_eq!(banner_size(4, 2), (152.0, 76.0));
        // Art wider than the space gets pixels under one screen pixel.
        let (w, h) = banner_size(1000, 10);
        assert_eq!(w, BANNER_MAX.0);
        assert!(h < 10.0);
    }

    #[test]
    fn clouds_moon_and_stars_stay_below_the_banner() {
        // Without a banner, they sit where they always have.
        assert_eq!(in_band(CLOUD_BAND, 0.0, 0), 70.0);
        assert_eq!(in_band(CLOUD_BAND, 0.0, 79), 149.0);
        assert_eq!(in_band(STAR_BAND, 0.0, 99), 119.0);
        assert_eq!(moon_y(0.0), MOON_Y);
        // With the tallest banner, everything starts below its bottom.
        let floor = BANNER_TOP + BANNER_MAX.1 + BANNER_GAP;
        for hash in 0..1000 {
            let cloud = in_band(CLOUD_BAND, floor, hash);
            assert!(cloud >= floor && cloud < CLOUD_BAND.1, "{cloud}");
            assert!(in_band(STAR_BAND, floor, hash) >= floor);
        }
        assert!(moon_y(floor) - MOON_RADIUS >= floor);
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
