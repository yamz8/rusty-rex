# rusty-rex

The Chrome offline dinosaur game, rebuilt in Rust with macroquad as an
Omarchy desktop game: one binary, installed as an Arch package so it shows
up in the app launcher.

## Loop

- `bin/run` plays a debug build; `bin/build` makes `build/rusty-rex`.
- `bin/test` runs `cargo fmt --check`, clippy with warnings as errors, and
  `cargo test`. Run it before committing.
- `bin/install` runs `makepkg -fi` and needs sudo. Never hand it back to the
  user: run `omarchy-launch-floating-terminal-with-presentation "bin/install"`
  (absolute path) so they type the password in an Omarchy terminal.
- Cargo lives in `~/.cargo/bin` (rustup) and isn't on PATH in every shell;
  the `bin/` scripts source `~/.cargo/env` when needed.

## Where things are

- `src/main.rs` window setup and the game loop.
- `pkgbuild/` PKGBUILD, desktop entry, icon.
- `docs/roadmap.md` what's next.

## Conventions

- Keep game logic (physics, spawning, collisions, score) apart from drawing
  so it can be unit tested without a window.
- Comments explain why, in full sentences. Commit messages short and
  imperative.
- Don't change Omarchy or OS settings without asking.
