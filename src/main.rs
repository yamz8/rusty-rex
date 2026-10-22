mod autopilot;
mod banner;
mod clock;
mod draw;
mod font;
mod game;
mod gfx;
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
use miniquad::conf::{Conf, LinuxBackend, Platform};
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton};
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

/// Keys and buttons held, and those that went down since the last frame.
#[derive(Default)]
struct Keys {
    down: Vec<KeyCode>,
    pressed: Vec<KeyCode>,
    click_down: bool,
    /// Where the left button last went down this frame, in window pixels.
    clicked: Option<(f32, f32)>,
    /// Any key or button at all went down, which ends the screensaver.
    any: bool,
}

impl Keys {
    fn is_down(&self, key: KeyCode) -> bool {
        self.down.contains(&key)
    }

    fn was_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    fn key_down(&mut self, key: KeyCode, repeat: bool) {
        self.any = true;
        // Held keys repeat, but only the first press counts as one.
        if !repeat && !self.is_down(key) {
            self.pressed.push(key);
            self.down.push(key);
        }
    }

    fn key_up(&mut self, key: KeyCode) {
        self.down.retain(|&k| k != key);
    }

    /// Forgets what is held, for when the window can no longer see keys
    /// being let go.
    fn release_all(&mut self) {
        self.down.clear();
        self.click_down = false;
    }

    /// Uses up this frame's click, so it neither presses nor holds jump.
    fn take_click(&mut self) {
        self.clicked = None;
        self.click_down = false;
    }

    fn end_frame(&mut self) {
        self.pressed.clear();
        self.clicked = None;
        self.any = false;
    }

    fn input(&self) -> Input {
        Input {
            // A left click jumps too, so a mouse is enough to play.
            jump: self.is_down(KeyCode::Space) || self.is_down(KeyCode::Up) || self.click_down,
            duck: self.is_down(KeyCode::Down),
            jump_pressed: self.was_pressed(KeyCode::Space)
                || self.was_pressed(KeyCode::Up)
                || self.clicked.is_some(),
            restart: self.was_pressed(KeyCode::Enter)
                || self.was_pressed(KeyCode::KpEnter)
                || self.clicked.is_some(),
        }
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
            // Chrome's button sound plays for jumps and restarts alike.
            Event::Jump | Event::Restart => &self.jump,
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
    let mode = options.mode;
    miniquad::start(window_conf(backend, mode), move || {
        Box::new(Stage::new(mode, banner))
    });
}

struct Stage {
    mode: Mode,
    gfx: gfx::Gfx,
    renderer: draw::Renderer,
    game: Game,
    session: Session,
    keys: Keys,
    /// None in the silent screensaver, which never starts the audio stack.
    sounds: Option<Sounds>,
    /// Muting lasts until the game closes.
    muted: bool,
    theme: Theme,
    score_path: Option<PathBuf>,
    score_saver: storage::ScoreSaver,
    /// The window lost or regained focus since the last frame.
    focus_lost: bool,
    focus_back: bool,
    /// The run was paused because the window lost focus, so it resumes
    /// when focus comes back.
    paused_for_focus: bool,
    /// When the window opened, as a date; times below count from it.
    started: f64,
    previous_frame: f64,
}

impl Stage {
    fn new(mode: Mode, banner: Option<sprites::Sprite>) -> Self {
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
        if screensaver {
            miniquad::window::show_mouse(false);
        }
        Self {
            mode,
            gfx: gfx::Gfx::new(),
            renderer: draw::Renderer::new(banner),
            score_saver: storage::ScoreSaver::new(game.high_score),
            game,
            session: Session::default(),
            keys: Keys::default(),
            sounds: (!screensaver).then(Sounds::load),
            muted: false,
            theme: Theme::new(),
            score_path,
            focus_lost: false,
            focus_back: false,
            paused_for_focus: false,
            started: miniquad::date::now(),
            previous_frame: 0.0,
        }
    }

    fn frame(&mut self) {
        let screensaver = self.mode == Mode::Screensaver;
        // The high score is already saved by the frame that set it.
        if self.keys.was_pressed(KeyCode::Escape) || (screensaver && self.keys.any) {
            miniquad::window::order_quit();
            return;
        }
        let now = miniquad::date::now() - self.started;
        let elapsed = (now - self.previous_frame) as f32;
        self.previous_frame = now;
        let game = &mut self.game;
        if screensaver {
            self.session.autoplay(game, elapsed);
        } else {
            // A click on the high score at game over is for resetting it,
            // not for playing again.
            if let Some(click) = self.keys.clicked
                && matches!(game.state, State::Over(_))
                && self.renderer.on_high_score(view::playfield_point(
                    miniquad::window::screen_size(),
                    click,
                ))
            {
                game.click_high_score();
                self.keys.take_click();
            }
            // Like Chrome, stop when the window loses focus and carry on
            // when it comes back.
            let mut pause = self.keys.was_pressed(KeyCode::P);
            // Focus lost during the intro pauses the run as soon as it
            // starts, unless focus is back by then.
            if self.focus_lost && game.state == State::Running {
                pause = !pause;
                self.paused_for_focus = true;
            }
            if (self.focus_lost && !matches!(game.state, State::Intro(_))) || self.focus_back {
                self.focus_lost = false;
            }
            if std::mem::take(&mut self.focus_back)
                && std::mem::take(&mut self.paused_for_focus)
                && game.state == State::Paused
            {
                pause = !pause;
            }
            self.session
                .advance(game, elapsed, self.keys.input(), pause);
            if game.state != State::Paused {
                self.paused_for_focus = false;
            }
        }
        if self.keys.was_pressed(KeyCode::M) {
            self.muted = !self.muted;
        }
        for event in game.events.drain(..) {
            if let Some(sounds) = self.sounds.as_ref().filter(|_| !self.muted) {
                sounds.play(event);
            }
        }
        // Losing a high score isn't worth crashing the game over.
        if let Some(path) = &self.score_path
            && let Err(e) = self.score_saver.save_if_needed(path, game.high_score, now)
        {
            eprintln!(
                "rusty-rex: couldn't save high score to {}: {e}",
                path.display()
            );
        }
        self.theme.refresh(now);
        self.renderer.frame(
            &mut self.gfx,
            miniquad::window::screen_size(),
            game,
            self.theme.palette,
            self.muted,
            now,
        );
        // Nobody plays the screensaver, so half the display's frame rate is
        // plenty, and it halves the GPU time. Waiting, paused and game-over
        // screens barely move.
        if screensaver || !game.is_live() {
            // Miniquad 0.4.11 cannot wake a blocked X11 event loop from a
            // timer. A short idle sleep also keeps theme polling and save
            // retries working, without a platform-specific wakeup thread.
            let spent = miniquad::date::now() - self.started - now;
            let remaining = (IDLE_FRAME_SECONDS - spent).max(0.0);
            std::thread::sleep(std::time::Duration::from_secs_f64(remaining));
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        self.frame();
        self.keys.end_frame();
    }

    fn key_down_event(&mut self, key: KeyCode, _: KeyMods, repeat: bool) {
        self.keys.key_down(key, repeat);
    }

    fn key_up_event(&mut self, key: KeyCode, _: KeyMods) {
        self.keys.key_up(key);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.keys.any = true;
        if button == MouseButton::Left {
            self.keys.click_down = true;
            self.keys.clicked = Some((x, y));
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _: f32, _: f32) {
        if button == MouseButton::Left {
            self.keys.click_down = false;
        }
    }

    /// On X11 miniquad reports losing focus as minimising.
    fn window_minimized_event(&mut self) {
        self.keys.release_all();
        self.focus_lost = true;
    }

    fn window_restored_event(&mut self) {
        self.focus_back = true;
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
    fn a_click_used_up_does_not_hold_jump() {
        let mut keys = Keys {
            click_down: true,
            clicked: Some((1.0, 2.0)),
            ..Keys::default()
        };
        assert!(keys.input().jump && keys.input().restart);
        keys.take_click();
        let input = keys.input();
        assert!(!input.jump && !input.jump_pressed && !input.restart);
    }

    #[test]
    fn no_display_has_no_backend() {
        assert_eq!(linux_backend(false, false), None);
    }
}
