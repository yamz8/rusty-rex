mod autopilot;
mod banner;
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
use macroquad::miniquad::conf::{LinuxBackend, Platform};
use macroquad::prelude::*;
use palette::Palette;
use rex::Input;
use session::Session;
use std::path::{Path, PathBuf};
use view::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// How often to check whether the Omarchy theme changed.
const THEME_CHECK_SECONDS: f64 = 1.0;
/// Bound idle and screensaver rendering without blocking input or theme
/// checks for long.
const IDLE_FRAME_SECONDS: f64 = 1.0 / 30.0;

/// Omarchy's screensaver windows share this class, which its window rules
/// make fullscreen and its idle service watches.
const SCREENSAVER_CLASS: &str = "org.omarchy.screensaver";

const HELP: &str = "\
Rusty Rex: the offline dinosaur runner, in Rust.

Usage: rusty-rex [--screensaver] [--banner FILE]

  --screensaver  Play by itself, fullscreen and silent, until any key or click
  --banner FILE  Draw the half-block art (▄ █ ▀) in FILE faintly in the sky
  -h, --help     Show this help
  -V, --version  Show the version

Space, Up or a click jumps; hold for a higher jump. Down ducks.
P pauses, M mutes and Esc quits.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Play,
    /// The rex plays itself, silently and without touching the high score,
    /// until any key or click.
    Screensaver,
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    mode: Mode,
    banner: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run(Options),
    Help,
    Version,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut options = Options {
        mode: Mode::Play,
        banner: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screensaver" => options.mode = Mode::Screensaver,
            "--banner" => {
                let file = args.next().ok_or("--banner needs a file")?;
                options.banner = Some(file.into());
            }
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            _ => return Err(format!("unknown argument {arg}; see rusty-rex --help")),
        }
    }
    Ok(Command::Run(options))
}

/// Reads the banner art. A missing or blank banner is only worth a warning:
/// the screensaver must still start.
fn load_banner(path: &Path) -> Option<sprites::Sprite> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let art = banner::parse(&text);
            if art.is_none() {
                eprintln!("rusty-rex: no ▄ █ ▀ blocks to draw in {}", path.display());
            }
            art
        }
        Err(e) => {
            eprintln!("rusty-rex: couldn't read banner {}: {e}", path.display());
            None
        }
    }
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

/// The sound effects and the audio thread that plays them. Creating this
/// starts the audio stack, so the silent screensaver never does.
struct Sounds {
    context: quad_snd::AudioContext,
    jump: quad_snd::Sound,
    milestone: quad_snd::Sound,
    crash: quad_snd::Sound,
}

impl Sounds {
    fn load() -> Self {
        let context = quad_snd::AudioContext::new();
        let load = |notes| quad_snd::Sound::load(&context, &sound::wav(notes));
        Self {
            jump: load(sound::JUMP),
            milestone: load(sound::MILESTONE),
            crash: load(sound::CRASH),
            context,
        }
    }

    fn play(&self, event: Event) {
        let sound = match event {
            Event::Jump => &self.jump,
            Event::Milestone => &self.milestone,
            Event::Crash => &self.crash,
        };
        sound.play(&self.context, quad_snd::PlaySoundParams::default());
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
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(Command::Run(options)) => options,
        Ok(Command::Help) => {
            println!("{HELP}");
            return;
        }
        Ok(Command::Version) => {
            println!("rusty-rex {}", env!("CARGO_PKG_VERSION"));
            return;
        }
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
    let banner = options.banner.as_deref().and_then(load_banner);
    macroquad::Window::from_config(
        window_conf(backend, options.mode),
        run(options.mode, banner),
    );
}

async fn run(mode: Mode, banner: Option<sprites::Sprite>) {
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
    let sounds = (!screensaver).then(Sounds::load);
    let mut theme = Theme::new();
    // Muting lasts until the game closes.
    let mut muted = false;
    let mut session = Session::default();
    let mut renderer = draw::Renderer::new(banner);
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
        // Nobody plays the screensaver, so half the display's frame rate is
        // plenty, and it halves the GPU time. Waiting, paused and game-over
        // screens barely move.
        if screensaver || game.state != State::Running {
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

    fn args(list: &[&str]) -> Result<Command, String> {
        parse_args(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_the_command_line() {
        let run = |mode, banner: Option<&str>| {
            Ok(Command::Run(Options {
                mode,
                banner: banner.map(PathBuf::from),
            }))
        };
        assert_eq!(args(&[]), run(Mode::Play, None));
        assert_eq!(args(&["--screensaver"]), run(Mode::Screensaver, None));
        assert_eq!(
            args(&["--banner", "logo.txt", "--screensaver"]),
            run(Mode::Screensaver, Some("logo.txt"))
        );
        assert!(args(&["--banner"]).is_err());
        assert_eq!(args(&["--screensaver", "-h"]), Ok(Command::Help));
        assert_eq!(args(&["--version"]), Ok(Command::Version));
        assert!(args(&["--screen"]).is_err());
    }

    #[test]
    fn no_display_has_no_backend() {
        assert_eq!(linux_backend(false, false), None);
    }
}
