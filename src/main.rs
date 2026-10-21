mod clock;
mod draw;
mod game;
mod obstacle;
mod palette;
mod rex;
mod sound;
mod storage;
mod theme;
mod view;

use clock::{Clock, TICK};
use game::{Event, Game};
use macroquad::audio::{Sound, load_sound_from_bytes, play_sound_once};
use macroquad::prelude::*;
use palette::Palette;
use rex::Input;
use std::path::PathBuf;
use view::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// How often to check whether the Omarchy theme changed.
const THEME_CHECK_SECONDS: f64 = 1.0;

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

struct Sounds {
    jump: Sound,
    milestone: Sound,
    crash: Sound,
}

impl Sounds {
    /// None when a sound can't be loaded; the game is still playable silent.
    async fn load() -> Option<Self> {
        Some(Self {
            jump: load_sound_from_bytes(&sound::wav(sound::JUMP)).await.ok()?,
            milestone: load_sound_from_bytes(&sound::wav(sound::MILESTONE))
                .await
                .ok()?,
            crash: load_sound_from_bytes(&sound::wav(sound::CRASH))
                .await
                .ok()?,
        })
    }

    fn play(&self, event: Event) {
        play_sound_once(match event {
            Event::Jump => &self.jump,
            Event::Milestone => &self.milestone,
            Event::Crash => &self.crash,
        });
    }
}

/// Follows the Omarchy theme, falling back to Chrome's colours when there is
/// no theme file (or it can't be read), so the game also runs outside Omarchy.
struct Theme {
    path: Option<PathBuf>,
    /// The file's last contents, so an unchanged theme isn't parsed again.
    text: Option<String>,
    palette: Palette,
    checked_at: f64,
}

impl Theme {
    fn new() -> Self {
        let mut theme = Self {
            path: theme::colors_path(std::env::var_os("HOME")),
            text: None,
            palette: palette::CHROME,
            checked_at: f64::NEG_INFINITY,
        };
        theme.refresh(0.0);
        theme
    }

    /// Rereads the file at most once every THEME_CHECK_SECONDS. Reading a
    /// small file is cheaper and more reliable than watching modification
    /// times, since Omarchy may swap the whole theme directory.
    fn refresh(&mut self, now: f64) {
        if now - self.checked_at < THEME_CHECK_SECONDS {
            return;
        }
        self.checked_at = now;
        let text = self
            .path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok());
        if text != self.text {
            self.palette = text
                .as_deref()
                .and_then(theme::parse)
                .unwrap_or(palette::CHROME);
            self.text = text;
        }
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
    let sounds = Sounds::load().await;
    if sounds.is_none() {
        eprintln!("rusty-rex: couldn't load sounds, playing silent");
    }
    let mut theme = Theme::new();
    let mut clock = Clock::default();
    loop {
        let input = read_input();
        for _ in 0..clock.advance(get_frame_time()) {
            game.update(TICK, input);
        }
        for event in game.events.drain(..) {
            if let Some(sounds) = &sounds {
                sounds.play(event);
            }
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
        theme.refresh(get_time());
        draw::frame(&game, theme.palette);
        next_frame().await;
    }
}
