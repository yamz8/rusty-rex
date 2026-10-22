//! The rex's movement: running, jumping and ducking.
//!
//! Nothing here touches the window, so the physics can be tested without a
//! window. Heights are in pixels above the ground and times in seconds.

// Chrome's numbers are per frame at 60 fps; these are the same per second.

/// Pulls the rex back down while airborne: Chrome's 0.6 px/frame².
pub const GRAVITY: f32 = 2160.0;
/// Upward speed at take-off before the game speed is added: 10 px/frame.
pub const JUMP_VELOCITY: f32 = 600.0;
/// Each px/s of game speed adds this much take-off speed, so faster runs jump
/// harder, as in Chrome (a tenth of the speed in px/frame).
pub const JUMP_SPEED_BONUS: f32 = 0.1;
/// Ending a jump caps the rising speed at this, 5 px/frame.
pub const DROP_VELOCITY: f32 = 300.0;
/// Letting go of jump only ends it once the rex is this high. Chrome ignores
/// an earlier release, so a very quick tap still makes a full jump.
pub const MIN_JUMP_HEIGHT: f32 = 30.0;
/// Every jump ends here, so a held jump tops out near 84 px.
pub const MAX_JUMP_HEIGHT: f32 = 63.0;
/// Down in the air turns the jump into a fall at once, at this speed (1
/// px/frame), and the fall then goes this many times faster.
pub const SPEED_DROP_VELOCITY: f32 = 60.0;
pub const SPEED_DROP_FACTOR: f32 = 3.0;
/// How long each leg frame lasts: Chrome runs at 12 frames a second and
/// ducks at 8.
pub const STEP_TIME: f32 = 1.0 / 12.0;
pub const DUCK_STEP_TIME: f32 = 1.0 / 8.0;

pub const STANDING_SIZE: (f32, f32) = (44.0, 47.0);
pub const DUCKING_SIZE: (f32, f32) = (59.0, 30.0);

/// The keys the rex cares about this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub jump: bool,
    pub duck: bool,
    /// Jump went down since the last physics tick, even if it has been
    /// released again. The rex only takes off on this, never on a held key.
    pub jump_pressed: bool,
    /// Enter or a click: restarts after a crash without waiting.
    pub restart: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Running,
    Jumping,
    Ducking,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rex {
    /// Height of the rex's feet above the ground.
    pub height: f32,
    /// Vertical speed, positive upwards.
    pub velocity: f32,
    pub pose: Pose,
    /// The jump has risen past MIN_JUMP_HEIGHT, so it can be ended.
    reached_min_height: bool,
    /// Down was pressed in the air, cutting the jump into a fast fall.
    speed_drop: bool,
    /// Jump was held last tick, to see the moment it is let go.
    jump_held: bool,
    step_timer: f32,
    /// Whether the back leg is lifted rather than the front one; flips every
    /// STEP_TIME while on the ground.
    pub back_leg_up: bool,
}

impl Default for Rex {
    fn default() -> Self {
        Self::new()
    }
}

impl Rex {
    pub fn new() -> Self {
        Self {
            height: 0.0,
            velocity: 0.0,
            pose: Pose::Running,
            reached_min_height: false,
            speed_drop: false,
            jump_held: false,
            step_timer: 0.0,
            back_leg_up: true,
        }
    }

    pub fn on_ground(&self) -> bool {
        self.pose != Pose::Jumping
    }

    /// Width and height of the rex's box for its current pose.
    pub fn size(&self) -> (f32, f32) {
        match self.pose {
            Pose::Ducking => DUCKING_SIZE,
            _ => STANDING_SIZE,
        }
    }

    /// Advances the rex by `dt` seconds while the game runs at `speed` px/s.
    pub fn update(&mut self, dt: f32, input: Input, speed: f32) {
        let released = self.jump_held && !input.jump;
        self.jump_held = input.jump;

        if self.on_ground() {
            // Chrome won't jump while ducking: down has to be let go first.
            if input.duck && (self.pose == Pose::Ducking || !input.jump_pressed) {
                self.pose = Pose::Ducking;
            } else if input.jump_pressed {
                self.pose = Pose::Jumping;
                self.velocity = JUMP_VELOCITY + speed * JUMP_SPEED_BONUS;
                self.reached_min_height = false;
                self.speed_drop = false;
            } else {
                self.pose = Pose::Running;
            }
        } else {
            if released {
                self.end_jump();
            }
            if input.duck && !self.speed_drop {
                self.speed_drop = true;
                self.velocity = -SPEED_DROP_VELOCITY;
            } else if !input.duck {
                self.speed_drop = false;
            }
        }

        if self.pose == Pose::Jumping {
            let factor = if self.speed_drop {
                SPEED_DROP_FACTOR
            } else {
                1.0
            };
            self.height += self.velocity * factor * dt;
            self.velocity -= GRAVITY * dt;
            if self.height > MIN_JUMP_HEIGHT || self.speed_drop {
                self.reached_min_height = true;
            }
            if self.height > MAX_JUMP_HEIGHT || self.speed_drop {
                self.end_jump();
            }
            if self.height <= 0.0 {
                // The rest of the step after touchdown is spent running.
                let fall = -self.velocity * factor;
                let below = (-self.height / fall.max(f32::EPSILON)).clamp(0.0, dt);
                self.height = 0.0;
                self.velocity = 0.0;
                self.step_legs(below);
                // Land straight into a duck if down is held, so a fast drop
                // flows into ducking under the next obstacle.
                self.pose = if input.duck {
                    Pose::Ducking
                } else {
                    Pose::Running
                };
                self.speed_drop = false;
            }
        } else {
            self.step_legs(dt);
        }
    }

    /// Caps the rising speed, once the jump is high enough to be cut short.
    fn end_jump(&mut self) {
        if self.reached_min_height && self.velocity > DROP_VELOCITY {
            self.velocity = DROP_VELOCITY;
        }
    }

    fn step_legs(&mut self, dt: f32) {
        let step = if self.pose == Pose::Ducking {
            DUCK_STEP_TIME
        } else {
            STEP_TIME
        };
        self.step_timer += dt;
        while self.step_timer >= step {
            self.step_timer -= step;
            self.back_leg_up = !self.back_leg_up;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;
    const SPEED: f32 = 360.0;
    /// The tick jump goes down on.
    const PRESS: Input = Input {
        jump_pressed: true,
        jump: true,
        duck: false,
        restart: false,
    };
    /// Jump held after the press.
    const JUMP: Input = Input {
        jump_pressed: false,
        jump: true,
        duck: false,
        restart: false,
    };
    const DUCK: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: true,
        restart: false,
    };
    const NONE: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: false,
        restart: false,
    };

    /// Steps until the rex lands, returning the peak height and the time
    /// aloft. `input` gets the time since take-off.
    fn fly(rex: &mut Rex, input: impl Fn(f32) -> Input) -> (f32, f32) {
        let mut peak = rex.height;
        let mut time = 0.0;
        while !rex.on_ground() {
            rex.update(DT, input(time), SPEED);
            peak = peak.max(rex.height);
            time += DT;
            assert!(time < 5.0, "rex never landed");
        }
        (peak, time)
    }

    fn jumped() -> Rex {
        let mut rex = Rex::new();
        rex.update(DT, PRESS, SPEED);
        rex
    }

    #[test]
    fn starts_running_on_the_ground() {
        let rex = Rex::new();
        assert_eq!(rex.pose, Pose::Running);
        assert_eq!(rex.height, 0.0);
        assert_eq!(rex.size(), STANDING_SIZE);
    }

    #[test]
    fn stays_on_the_ground_without_input() {
        let mut rex = Rex::new();
        for _ in 0..240 {
            rex.update(DT, NONE, SPEED);
        }
        assert_eq!(rex.height, 0.0);
        assert_eq!(rex.pose, Pose::Running);
    }

    #[test]
    fn legs_alternate_at_chromes_rates() {
        let mut rex = Rex::new();
        let start = rex.back_leg_up;
        rex.update(STEP_TIME, NONE, SPEED);
        assert_ne!(rex.back_leg_up, start);

        let mut ducking = Rex::new();
        ducking.update(STEP_TIME, DUCK, SPEED);
        assert_eq!(ducking.back_leg_up, start, "ducking steps more slowly");
        ducking.update(DUCK_STEP_TIME - STEP_TIME, DUCK, SPEED);
        assert_ne!(ducking.back_leg_up, start);
    }

    #[test]
    fn a_held_jump_is_capped_near_84_px() {
        let mut rex = jumped();
        let (peak, time) = fly(&mut rex, |_| JUMP);
        assert!((80.0..88.0).contains(&peak), "peak {peak}");
        assert!((0.5..0.6).contains(&time), "aloft {time} s");
        assert_eq!(rex.height, 0.0);
        assert_eq!(rex.velocity, 0.0);
        assert_eq!(rex.pose, Pose::Running);
    }

    #[test]
    fn faster_runs_take_off_harder() {
        let mut slow = Rex::new();
        slow.update(DT, PRESS, 360.0);
        let mut fast = Rex::new();
        fast.update(DT, PRESS, 780.0);
        assert!(fast.velocity > slow.velocity);
        // Chrome: 10 + 13 / 10 px/frame at top speed.
        assert!((fast.velocity + GRAVITY * DT - 678.0).abs() < 0.01);
    }

    #[test]
    fn letting_go_past_the_minimum_gives_a_short_hop() {
        let mut full = jumped();
        let (full_peak, _) = fly(&mut full, |_| JUMP);

        // Let go just past the minimum height, a quick tap.
        let mut hop = jumped();
        let (hop_peak, _) = fly(&mut hop, |t| if t < 0.06 { JUMP } else { NONE });
        assert!(
            hop_peak < full_peak - 15.0,
            "hop {hop_peak}, full {full_peak}"
        );
        assert!(hop_peak > MIN_JUMP_HEIGHT);
    }

    #[test]
    fn letting_go_too_soon_is_ignored_as_in_chrome() {
        let mut full = jumped();
        let (full_peak, _) = fly(&mut full, |_| JUMP);
        let mut tap = jumped();
        let (tap_peak, _) = fly(&mut tap, |_| NONE);
        assert!(
            (tap_peak - full_peak).abs() < 1.0,
            "{tap_peak} vs {full_peak}"
        );
    }

    #[test]
    fn holding_jump_does_not_bounce_after_landing() {
        let mut rex = jumped();
        fly(&mut rex, |_| JUMP);
        for _ in 0..60 {
            rex.update(DT, JUMP, SPEED);
            assert_eq!(rex.pose, Pose::Running);
        }

        rex.update(DT, PRESS, SPEED);
        assert_eq!(rex.pose, Pose::Jumping);
    }

    #[test]
    fn cannot_jump_again_in_mid_air() {
        let mut rex = jumped();
        while rex.velocity > 0.0 {
            rex.update(DT, JUMP, SPEED);
        }
        let before = rex.velocity;
        rex.update(DT, PRESS, SPEED);
        assert!(rex.velocity < before, "jump re-triggered in the air");
    }

    #[test]
    fn down_in_the_air_falls_at_once_and_fast() {
        let mut normal = jumped();
        let (_, normal_time) = fly(&mut normal, |_| JUMP);

        let mut fast = jumped();
        fast.update(DT, JUMP, SPEED);
        assert!(fast.velocity > 0.0);
        fast.update(DT, DUCK, SPEED);
        assert!(fast.velocity < 0.0, "still rising after down");
        let (_, fast_time) = fly(&mut fast, |_| DUCK);
        assert!(fast_time < normal_time / 2.0);
    }

    #[test]
    fn landing_with_down_held_ducks() {
        let mut rex = jumped();
        fly(&mut rex, |_| DUCK);
        assert_eq!(rex.pose, Pose::Ducking);
    }

    #[test]
    fn ducks_while_down_is_held_and_stands_on_release() {
        let mut rex = Rex::new();
        rex.update(DT, DUCK, SPEED);
        assert_eq!(rex.pose, Pose::Ducking);
        assert_eq!(rex.size(), DUCKING_SIZE);
        assert_eq!(rex.height, 0.0);

        rex.update(DT, NONE, SPEED);
        assert_eq!(rex.pose, Pose::Running);
        assert_eq!(rex.size(), STANDING_SIZE);
    }

    #[test]
    fn no_jumping_while_ducking() {
        let mut rex = Rex::new();
        rex.update(DT, DUCK, SPEED);
        let both = Input {
            jump_pressed: true,
            jump: true,
            duck: true,
            restart: false,
        };
        rex.update(DT, both, SPEED);
        assert_eq!(rex.pose, Pose::Ducking);
        // Letting go of down frees the jump.
        rex.update(DT, PRESS, SPEED);
        assert_eq!(rex.pose, Pose::Jumping);
    }

    #[test]
    fn time_after_touchdown_goes_to_running() {
        let mut rex = Rex {
            pose: Pose::Jumping,
            height: 1.0,
            velocity: -240.0,
            reached_min_height: true,
            ..Rex::new()
        };
        rex.update(DT, JUMP, SPEED);
        assert_eq!(rex.pose, Pose::Running);
        // Falling at 240 px/s from 1 px, it lands 1/240 s into the step, so
        // the remaining 1/120 - 1/240 s goes to the legs.
        let expected = DT - 1.0 / 240.0;
        assert!(
            (rex.step_timer - expected).abs() < 1e-3,
            "{}",
            rex.step_timer
        );
    }
}
