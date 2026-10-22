# Roadmap

- [x] Rex that runs and jumps (space / up), ducks (down)
- [x] Ground scrolling and speed that ramps up
- [x] Cacti and pterodactyls, collision, game over and restart
- [x] Score and high score (saved in ~/.local/share/rusty-rex)
- [x] Day/night flip, sounds
- [x] Follow the Omarchy theme colours
- [x] Clouds drifting past at a fifth of the ground speed
- [x] Pause (P) and mute (M)
- [x] Esc to quit, and a click jumps like Space
- [x] Moon phases that change each night, as in Chrome
- [x] Rex blinks now and then on the start screen
- [x] `--screensaver`: the rex plays by itself until any key or click
- [x] `--banner FILE`: half-block text art drawn faintly in the sky
- [x] Parity with Chrome's physics, obstacles, score, intro, restart and
      night sky; see [parity.md](parity.md)
- [x] Draw on miniquad directly, silent screensaver without an audio stack
- [ ] `--rex FILE`: swap the runner for half-block art, keeping the rex's
      hitbox whatever is drawn
- [ ] Memoise the autopilot's look-ahead if its CPU cost ever shows; it
      branches at every landing and obstacle, about 1% CPU in practice
