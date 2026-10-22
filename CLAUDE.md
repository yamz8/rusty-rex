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
- The `bin/` scripts share `bin/cargo-env`, which checks that Cargo works
  and falls back to `~/.cargo/bin` when PATH contains an unconfigured shim.

## Releasing

1. Bump the version in `Cargo.toml` and `pkgbuild/PKGBUILD`, run
   `bin/test`, commit "Release X.Y.Z". Any code change after a release
   needs a new version, so one version never names two different builds.
2. Tag `vX.Y.Z`, push the commit and tag, then `gh release create` with
   `build/rusty-rex` packed with the desktop entry, icon, LICENSE, NOTICE
   and README.
3. Only now update the AUR package, since its checksum is of GitHub's tarball
   for the pushed tag: set `pkgver` and the sha256 of
   `$url/archive/vX.Y.Z.tar.gz` in `pkgbuild/aur/PKGBUILD`, regenerate
   `.SRCINFO` with `makepkg --printsrcinfo`, test with `makepkg -f --nodeps`
   in a scratch directory (cargo is from rustup, not pacman), commit "Update
   the AUR package to X.Y.Z" and push. Then push both files to the AUR repo.

## Where things are

- `src/main.rs` window setup and the game loop, plus the glue that loads
  sounds, the high score and the theme.
- Game logic, no macroquad, unit tested:
  - `src/rex.rs` running, jumping and ducking physics.
  - `src/game.rs` states, scrolling, speed, spawning, collisions, score,
    night and sound events.
  - `src/obstacle.rs` cacti and pterodactyls, hitboxes, the seeded RNG.
  - `src/clock.rs` fixed 1/120 s physics ticks.
  - `src/session.rs` buffered input, fixed ticks and idle/resume timing.
  - `src/autopilot.rs` plays by itself for `--screensaver`, trying each
    move on copies of the game.
  - `src/view.rs` fitting the 960x320 playfield into any window size.
  - `src/palette.rs` colours and the day/night blend.
  - `src/theme.rs` reads the Omarchy theme's `colors.toml`.
  - `src/storage.rs` the high score file.
  - `src/sound.rs` synthesises the sound effects as WAVs.
  - `src/sprites.rs` pixel art as text grids.
- `src/draw.rs` all drawing; it only reads game state.
- `pkgbuild/` PKGBUILD for `bin/install`, desktop entry, icon;
  `pkgbuild/aur/` the AUR package, built from GitHub release tags.
- `.github/workflows/test.yml` runs `bin/test` on every push.
- `docs/roadmap.md` what's next.

## Conventions

- Keep game logic (physics, spawning, collisions, score) apart from drawing
  so it can be unit tested without a window.
- Comments explain why, in full sentences. Commit messages short and
  imperative.
- Don't change Omarchy or OS settings without asking.
