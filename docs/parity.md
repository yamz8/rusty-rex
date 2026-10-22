# Parity with Chrome

How Rusty Rex compares with Chrome's offline dino game, feature by feature.
The reference is Chromium's `components/neterror/resources/dino_game/`
(main branch, read October 2026). Chrome's file names below drop the `.ts`.

Chrome counts in pixels per frame at 60 fps on a 600×150 canvas. Rusty Rex
counts in pixels per second on a 960×320 playfield, with sprites at the
same pixel sizes, so 1 px/frame is 60 px/s and 1 px/frame² is 3600 px/s².

**Status**: *same* matches Chrome; *close* matches in substance, with the
difference noted; *differs* is on purpose, with the reason; *skipped* is
left out on purpose; *extra* is Rusty Rex's own.

## The rex

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Gravity | 0.6 px/frame² (`trex`) | 2160 px/s² (`rex.rs`) | same |
| Take-off | `-10 - speed/10` px/frame, so faster runs jump harder | 600 px/s plus a tenth of the speed (`rex.rs`) | same |
| Jump height | Every jump ends 63 px up (canvas y 30): rising slows to 5 px/frame, for a peak near 84 px | Same (`rex.rs`) | same |
| Short hop | Letting go caps rising at 5 px/frame, but only once the rex is 30 px up; an earlier release is ignored | Same (`rex.rs`) | same |
| Fast drop | Down in the air sets the speed to 1 px/frame falling, then falls 3× as fast | Same (`rex.rs`) | same |
| Land into a duck | Still holding down on landing ducks | Same (`rex.rs`) | same |
| Duck | 59 wide, 8 fps; no jump while ducking | Same (`rex.rs`); the duck is drawn 30 tall to Chrome's 25 | close |
| Run animation | 2 frames at 12 fps | Same (`rex.rs`) | same |
| Crash sprite | The standing crashed rex, even when it crashed ducking | Same (`sprites.rs`, `draw.rs`) | same |
| Holding jump | OS key repeat makes the rex jump again on landing | A fresh press is needed for each jump (`game.rs`) | differs: a held key that keeps jumping feels like a bug |
| Keys | Space and Up jump, Down ducks, Enter restarts | Same, plus P pauses, M mutes and Esc quits (`main.rs`) | same |
| Mouse | On desktop a click only restarts after a crash | A left click also jumps (`main.rs`) | differs: with no touch screen, a click is how a mouse user plays |
| Touch | Tapping jumps; letting go ends the jump | — | skipped: a desktop game |
| Gamepad | Arcade mode only: button 0 jumps, 1 ducks, 9 restarts | — | skipped |
| Position | 50 px from the left of a 600 px canvas | 60 px from the left of 960 (`game.rs`) | differs: the wider playfield |

## Speed and timing

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Start speed | 6 px/frame | 360 px/s (`game.rs`) | same |
| Acceleration | +0.001 px/frame every frame | 3.6 px/s² (`game.rs`) | same |
| Top speed | 13 px/frame | 780 px/s (`game.rs`) | same |
| Clear time | No obstacles in the first 3 s of running | Same (`game.rs`) | same |
| Small screens | Narrower than 600 px slows the game | — | skipped: the playfield scales instead (`view.rs`) |
| Frame stepping | Moves by each frame's elapsed time; acceleration is per frame, so faster on high-refresh screens | Fixed 120 Hz ticks (`clock.rs`, `session.rs`) | differs: the same game at any refresh rate |

## Obstacles

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Kinds and sizes | Small cactus 17×35, large 25×50, pterodactyl 46×40 | Same (`obstacle.rs`) | same |
| Picking | Uniform over the kinds allowed at this speed; never 3 of a kind in a row | Same (`obstacle.rs`) | same |
| Groups | 1–3 cacti once the game reaches 4 (small) or 7 (large) px/frame | Same (`obstacle.rs`) | same |
| Pterodactyls | From 8.5 px/frame, underside 0, 25 or 50 px up | Same (`obstacle.rs`) | same |
| Pterodactyl speed | Each flies 0.8 px/frame faster or slower than the ground | Same (`obstacle.rs`, `game.rs`) | same |
| Flapping | 2 frames at 6 fps | Same (`obstacle.rs`) | same |
| Gaps | `width × speed + minGap × 0.6`, up to 1.5× that; minGap 120, 120, 150; the next comes when the last is its gap clear of the edge | Same (`obstacle.rs`, `game.rs`) | same |
| Collision | A rough box less the 1 px border, then each rex box against each obstacle box: 6 for the running rex, 1 ducking, 3 per cactus (stretched across groups), 5 for a pterodactyl | Same boxes and order (`obstacle.rs`, `game.rs`) | same |
| Spawning | At the canvas's right edge | Past the right edge of the widest view (`game.rs`) | differs: wide windows show more than the playfield |
| Collectables | Only in alternate game modes | — | skipped: no alternate modes ship |

## Score

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Score | `round(ceil(distance) × 0.025)` | Same (`game.rs`) | same |
| Digits | 5, zero-padded; 6 past 99999, keeping the last six | Same, in a 5×7 pixel font 10 px wide (`draw.rs`, `font.rs`) | same |
| Milestones | Every 100: a chime, then the score blinks off and on 4 times, 250 ms each way, held at the milestone | Same (`game.rs`, `draw.rs`) | same |
| High score | `HI` and the best score, dimmed, to the left | Same, kept in `~/.local/share/rusty-rex` (`storage.rs`) | same |
| High score reset | At game over, click HI to make it blink, and click again while it blinks to reset it | Same (`game.rs`, `main.rs`) | same |

## Night

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Nightfall | Every 700 points, for 12 s | Same (`game.rs`) | same |
| Fade | The page inverts over 1.5 s on cubic-bezier(0.65, 0.05, 0.36, 1) | Same timing and curve (`game.rs`) | same |
| Colours | CSS invert of black on white | The Omarchy theme's colours, with a night version (`theme.rs`, `palette.rs`) | differs: it follows the desktop theme |
| Moon | 7 phases, one per night, drifting left 0.25 px/frame | Same; phases drawn as a shadowed circle (`draw.rs`, `game.rs`) | same |
| Moon on restart | Back to the first phase | Phases carry on across runs (`game.rs`) | differs: each night of a sitting gets a new moon |
| Stars | 2, drifting 0.3 px/frame, placed anew each night | Same (`draw.rs`, `sprites.rs`) | same |
| Dark mode | `prefers-color-scheme: dark` inverts the canvas | The theme's own colours | differs: see Colours |

## Sky and ground

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Clouds | 46×14, 100–400 px apart, at a steady 1 px/frame whatever the game speed, kept across restarts | Same speed, kept across restarts, spaced by hashed slots (`draw.rs`, `game.rs`) | close |
| Ground | A line in 600 px stretches, each flat or bumpy at random | Same, plus pebbles below the line (`draw.rs`) | same |
| Background scenery | Only in alternate game modes | — | skipped: no alternate modes ship |

## Starting, pausing and game over

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Waiting rex | Only the rex shows; it blinks after random waits of up to 7 s, three times | Same (`draw.rs`) | same |
| Intro | The first jump happens in place; on landing the game widens out from the rex over 0.4 s while the rex walks in, and the world starts moving after | Same (`game.rs`, `draw.rs`) | same |
| Prompt | Page text below the game | Drawn in the playfield before the first run and while paused (`draw.rs`) | differs: there is no page around the game |
| Game over | GAME OVER and a restart button that animates, with no other hint | Same text and button, the button still (`draw.rs`, `sprites.rs`) | close |
| Restart | Enter or a click at once, jump keys after 1.2 s, on key release; plays the button sound | Same, on key press (`game.rs`) | close |
| Losing focus | Switching tabs or windows stops the game; coming back carries on | Same where the window system reports focus: X11, the default (`main.rs`) | close |
| Pause key | — | P pauses and resumes | extra |

## Sound

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Effects | Recorded clips: the button press (jump and restart), the crash and the milestone | The same three, synthesised as square waves (`sound.rs`) | differs: no audio files to ship |
| Mute | — | M | extra |

## Accessibility and other modes

| Feature | Chrome | Rusty Rex | Status |
|---|---|---|---|
| Audio cues | With a screen reader: footsteps, a start jingle and a tone when an obstacle is near | — | skipped: needs screen-reader integration a game window doesn't have |
| Announcements | Start, game over and score read out through `aria-live` | — | skipped: same reason |
| Slow mode | Offered with audio cues: slower, gentler jumps, wider gaps | — | skipped: part of the audio-cue mode |
| Alternate game modes | Themed sprites and collectables, none shipped | — | skipped |
| Arcade mode | `chrome://dino` scales the game to fill the page | The game always scales to its window (`view.rs`) | same |
| Right-to-left pages | Mirrors the canvas | — | skipped |
| Screensaver | — | `--screensaver` plays by itself (`autopilot.rs`) | extra |
| Banner | — | `--banner FILE` draws block or braille art in the sky (`banner.rs`) | extra |
