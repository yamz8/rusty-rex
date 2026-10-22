# Rusty Rex

The offline dinosaur runner from Chrome, rebuilt in Rust with
[macroquad](https://macroquad.rs) for Omarchy.

![Rusty Rex running past cacti as day turns to night](docs/demo.gif)

## How to play

- **Space** or **Up** jumps; hold it for a higher jump, tap it for a hop.
- **Down** ducks under pterodactyls, or drops you faster mid-jump.
- **P** pauses, **M** mutes the sound.
- The high score is kept in `~/.local/share/rusty-rex`.
- Colours follow your Omarchy theme and change when you switch themes.

## Build and play

```
bin/run        # play a debug build
bin/build      # release binary at build/rusty-rex
bin/test       # fmt, clippy, tests
bin/install    # install as an Arch package (shows up in the app launcher)
```

Rust comes from `omarchy install dev-env rust`.
The scripts use a working Cargo from PATH, falling back to `~/.cargo/bin`
if a toolchain-manager shim is unconfigured. Builds and tests use `Cargo.lock`.

Waiting, paused and game-over screens cap rendering at 30 FPS, while still
handling input, resizing, theme changes and save retries.
Gameplay keeps its fixed 120 Hz physics ticks, including brief jump presses
between ticks.

## License

MIT
