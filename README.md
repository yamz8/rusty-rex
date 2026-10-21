# Rusty Rex

The offline dinosaur runner from Chrome, rebuilt in Rust with
[macroquad](https://macroquad.rs) for Omarchy.

## How to play

- **Space** or **Up** jumps; hold it for a higher jump, tap it for a hop.
- **Down** ducks under pterodactyls, or drops you faster mid-jump.
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

## License

MIT
