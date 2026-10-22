mod autopilot;
mod clock;
mod draw;
mod game;
mod obstacle;
mod palette;
mod rex;
mod session;
mod sound;
mod sprites;
mod storage;
mod theme;
mod view;

use game::{Event, Game, State};
use macroquad::audio::{Sound, load_sound_from_bytes, play_sound_once};
use macroquad::miniquad::conf::{LinuxBackend, Platform};
use macroquad::prelude::*;
use palette::Palette;
use rex::Input;
use session::Session;
use std::path::PathBuf;
use view::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// How often to check whether the Omarchy theme changed.
const THEME_CHECK_SECONDS: f64 = 1.0;
/// Bound idle rendering without blocking input or theme checks for long.
const IDLE_FRAME_SECONDS: f64 = 1.0 / 30.0;

/// Omarchy's screensaver windows share this class, which its window rules
/// make fullscreen and its idle service watches.
const SCREENSAVER_CLASS: &str = "org.omarchy.screensaver";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Play,
    /// The rex plays itself, silently and without touching the high score,
    /// until any key or click.
    Screensaver,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Mode, String> {
    let mut mode = Mode::Play;
    for arg in args {
        match arg.as_str() {
            "--screensaver" => mode = Mode::Screensaver,
            _ => {
                return Err(format!(
                    "unknown argument {arg}; usage: rusty-rex [--screensaver]"
                ));
            }
        }
    }
    Ok(mode)
}

fn window_conf(linux_backend: LinuxBackend, mode: Mode) -> Conf {
    Conf {
        window_title: "Rusty Rex".to_owned(),
        window_width: PLAYFIELD_WIDTH as i32,
        window_height: PLAYFIELD_HEIGHT as i32,
        fullscreen: mode == Mode::Screensaver,
        platform: Platform {
            linux_backend,
            // Playing matches the desktop entry's StartupWMClass, so docks
            // show the icon; the screensaver joins Omarchy's own windows.
            linux_wm_class: match mode {
                Mode::Play => "rusty-rex",
                Mode::Screensaver => SCREENSAVER_CLASS,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Picks how to open the window on Linux. X11 (XWayland) is miniquad's
/// stable backend, so it stays the choice whenever there is an X display;
/// without one it panics, so a Wayland-only session uses Wayland instead.
/// None means there is no display at all.
fn linux_backend(has_x11: bool, has_wayland: bool) -> Option<LinuxBackend> {
    match (has_x11, has_wayland) {
        (true, _) => Some(LinuxBackend::X11Only),
        (false, true) => Some(LinuxBackend::WaylandOnly),
        (false, false) => None,
    }
}

fn read_input() -> Input {
    Input {
        // A left click jumps too, as in Chrome.
        jump: is_key_down(KeyCode::Space)
            || is_key_down(KeyCode::Up)
            || is_mouse_button_down(MouseButton::Left),
        duck: is_key_down(KeyCode::Down),
        jump_pressed: is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Up)
            || is_mouse_button_pressed(MouseButton::Left),
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

/// Whether the viewer touched anything this frame, which ends the
/// screensaver. Moving the mouse doesn't count, as with Omarchy's own.
fn any_input() -> bool {
    get_last_key_pressed().is_some()
        || [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .into_iter()
            .any(is_mouse_button_pressed)
}

fn main() {
    let mode = match parse_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("rusty-rex: {e}");
            std::process::exit(2);
        }
    };
    let backend = linux_backend(
        std::env::var_os("DISPLAY").is_some(),
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
    );
    // Without a display miniquad panics, which aborts and leaves a core
    // dump that desktops report as a crash. A plain error says what's wrong.
    let Some(backend) = backend else {
        eprintln!("rusty-rex: no display found; run it from a graphical session");
        std::process::exit(1);
    };
    macroquad::Window::from_config(window_conf(backend, mode), run(mode));
}

async fn run(mode: Mode) {
    let screensaver = mode == Mode::Screensaver;
    // Any changing seed will do; it only decides the order of obstacles.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    let mut game = Game::new(seed);
    // The autopilot's scores aren't the player's.
    let score_path = storage::default_high_score_path().filter(|_| !screensaver);
    if let Some(path) = &score_path {
        game.high_score = storage::load(path);
    }
    let mut score_saver = storage::ScoreSaver::new(game.high_score);
    let sounds = if screensaver {
        None
    } else {
        Sounds::load().await
    };
    if sounds.is_none() && !screensaver {
        eprintln!("rusty-rex: couldn't load sounds, playing silent");
    }
    let mut theme = Theme::new();
    // Muting lasts until the game closes.
    let mut muted = false;
    let mut session = Session::default();
    let mut renderer = draw::Renderer::new();
    if screensaver {
        show_mouse(false);
    }
    let mut previous_frame = get_time();
    // Returning ends macroquad's main future, which closes the window. The
    // high score is already saved by the frame that set it.
    loop {
        if is_key_pressed(KeyCode::Escape) || (screensaver && any_input()) {
            break;
        }
        let now = get_time();
        let elapsed = (now - previous_frame) as f32;
        previous_frame = now;
        if screensaver {
            session.autoplay(&mut game, elapsed);
        } else {
            session.advance(&mut game, elapsed, read_input(), is_key_pressed(KeyCode::P));
        }
        if is_key_pressed(KeyCode::M) {
            muted = !muted;
        }
        for event in game.events.drain(..) {
            if let Some(sounds) = sounds.as_ref().filter(|_| !muted) {
                sounds.play(event);
            }
        }
        // Losing a high score isn't worth crashing the game over.
        if let Some(path) = &score_path
            && let Err(e) = score_saver.save_if_needed(path, game.high_score, now)
        {
            eprintln!(
                "rusty-rex: couldn't save high score to {}: {e}",
                path.display()
            );
        }
        theme.refresh(now);
        renderer.frame(&game, theme.palette, muted);
        if game.state != State::Running {
            // Miniquad 0.4.11 cannot wake a blocked X11 event loop from a
            // timer. A short idle sleep also keeps theme polling and save
            // retries working, without a platform-specific wakeup thread.
            let remaining = (IDLE_FRAME_SECONDS - (get_time() - now)).max(0.0);
            std::thread::sleep(std::time::Duration::from_secs_f64(remaining));
        }
        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_x11_and_falls_back_to_wayland() {
        assert_eq!(linux_backend(true, true), Some(LinuxBackend::X11Only));
        assert_eq!(linux_backend(true, false), Some(LinuxBackend::X11Only));
        assert_eq!(linux_backend(false, true), Some(LinuxBackend::WaylandOnly));
    }

    fn args(list: &[&str]) -> Result<Mode, String> {
        parse_args(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_the_screensaver_flag() {
        assert_eq!(args(&[]), Ok(Mode::Play));
        assert_eq!(args(&["--screensaver"]), Ok(Mode::Screensaver));
        assert!(args(&["--screen"]).is_err());
    }

    #[test]
    fn no_display_has_no_backend() {
        assert_eq!(linux_backend(false, false), None);
    }
}
