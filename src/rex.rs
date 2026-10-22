//! The rex's movement: running, jumping and ducking.
//!
//! Nothing here touches macroquad, so the physics can be tested without a
//! window. Heights are in pixels above the ground and times in seconds.

/// Pulls the rex back down while airborne.
pub const GRAVITY: f32 = 2400.0;
/// Upward speed at take-off; with full gravity this peaks at about 127 px.
pub const JUMP_VELOCITY: f32 = 780.0;
/// Releasing jump early caps the rising speed at this, giving a short hop,
/// the way the Chrome game lets you tap for small jumps.
pub const SHORT_JUMP_VELOCITY: f32 = 300.0;
/// The short-hop cap only applies above this height, so even a one-frame tap
/// lifts the rex high enough to clear a small cactus.
pub const MIN_JUMP_HEIGHT: f32 = 30.0;
/// Holding down in the air multiplies gravity so the rex drops quickly.
pub const FAST_DROP_FACTOR: f32 = 3.0;
/// How long each leg frame of the running animation lasts.
pub const STEP_TIME: f32 = 0.1;

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

    pub fn update(&mut self, dt: f32, input: Input) {
        if self.on_ground() {
            // Jump wins over duck so pressing both doesn't leave the rex stuck
            // crouching when the player clearly wants to clear an obstacle.
            if input.jump_pressed {
                self.pose = Pose::Jumping;
                self.velocity = JUMP_VELOCITY;
            } else if input.duck {
                self.pose = Pose::Ducking;
            } else {
                self.pose = Pose::Running;
            }
        }

        if self.pose == Pose::Jumping {
            if !input.jump && self.height >= MIN_JUMP_HEIGHT && self.velocity > SHORT_JUMP_VELOCITY
            {
                self.velocity = SHORT_JUMP_VELOCITY;
            }
            let gravity = if input.duck {
                GRAVITY * FAST_DROP_FACTOR
            } else {
                GRAVITY
            };
            self.velocity -= gravity * dt;
            self.height += self.velocity * dt;
            if self.height <= 0.0 {
                // The rex reached the ground partway through this step; the
                // rest of the step is spent running rather than dropped.
                let below = (self.height / self.velocity).clamp(0.0, dt);
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
            }
        } else {
            self.step_legs(dt);
        }
    }

    fn step_legs(&mut self, dt: f32) {
        self.step_timer += dt;
        while self.step_timer >= STEP_TIME {
            self.step_timer -= STEP_TIME;
            self.back_leg_up = !self.back_leg_up;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;
    /// The tick jump goes down on.
    const PRESS: Input = Input {
        jump_pressed: true,
        jump: true,
        duck: false,
    };
    /// Jump held after the press.
    const JUMP: Input = Input {
        jump_pressed: false,
        jump: true,
        duck: false,
    };
    const DUCK: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: true,
    };
    const NONE: Input = Input {
        jump_pressed: false,
        jump: false,
        duck: false,
    };

    /// Steps until the rex lands, returning the peak height and the time aloft.
    fn fly(rex: &mut Rex, input: Input) -> (f32, f32) {
        let mut peak = rex.height;
        let mut time = 0.0;
        while !rex.on_ground() {
            rex.update(DT, input);
            peak = peak.max(rex.height);
            time += DT;
            assert!(time < 5.0, "rex never landed");
        }
        (peak, time)
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
            rex.update(DT, NONE);
        }
        assert_eq!(rex.height, 0.0);
        assert_eq!(rex.pose, Pose::Running);
    }

    #[test]
    fn legs_alternate_while_running() {
        let mut rex = Rex::new();
        let start = rex.back_leg_up;
        rex.update(STEP_TIME, NONE);
        assert_ne!(rex.back_leg_up, start);
        rex.update(STEP_TIME, NONE);
        assert_eq!(rex.back_leg_up, start);
    }

    #[test]
    fn jump_rises_then_lands() {
        let mut rex = Rex::new();
        rex.update(DT, PRESS);
        assert_eq!(rex.pose, Pose::Jumping);
        assert!(rex.height > 0.0);

        let (peak, _) = fly(&mut rex, JUMP);
        let expected = JUMP_VELOCITY * JUMP_VELOCITY / (2.0 * GRAVITY);
        assert!(
            (peak - expected).abs() < 5.0,
            "peak {peak}, want ~{expected}"
        );
        assert_eq!(rex.height, 0.0);
        assert_eq!(rex.velocity, 0.0);
        assert_eq!(rex.pose, Pose::Running);
    }

    #[test]
    fn tapping_jump_gives_a_short_hop() {
        let mut full = Rex::new();
        full.update(DT, PRESS);
        let (full_peak, _) = fly(&mut full, JUMP);

        let mut tap = Rex::new();
        tap.update(DT, PRESS);
        let (tap_peak, _) = fly(&mut tap, NONE);

        assert!(
            tap_peak < full_peak / 2.0,
            "tap {tap_peak}, full {full_peak}"
        );
    }

    #[test]
    fn a_one_frame_tap_still_clears_the_minimum_height() {
        let mut rex = Rex::new();
        rex.update(DT, PRESS);
        let (peak, _) = fly(&mut rex, NONE);
        assert!(peak >= MIN_JUMP_HEIGHT, "peak {peak}");
    }

    #[test]
    fn holding_jump_does_not_bounce_after_landing() {
        let mut rex = Rex::new();
        rex.update(DT, PRESS);
        fly(&mut rex, JUMP);
        for _ in 0..60 {
            rex.update(DT, JUMP);
            assert_eq!(rex.pose, Pose::Running);
        }

        rex.update(DT, PRESS);
        assert_eq!(rex.pose, Pose::Jumping);
    }

    #[test]
    fn cannot_jump_again_in_mid_air() {
        let mut rex = Rex::new();
        rex.update(DT, PRESS);
        while rex.velocity > 0.0 {
            rex.update(DT, JUMP);
        }
        let before = rex.velocity;
        rex.update(DT, PRESS);
        assert!(rex.velocity < before, "jump re-triggered in the air");
    }

    #[test]
    fn holding_down_in_the_air_drops_faster() {
        let mut normal = Rex::new();
        normal.update(DT, PRESS);
        let (_, normal_time) = fly(&mut normal, JUMP);

        let mut fast = Rex::new();
        fast.update(DT, PRESS);
        let (_, fast_time) = fly(
            &mut fast,
            Input {
                jump_pressed: false,
                jump: true,
                duck: true,
            },
        );

        assert!(fast_time < normal_time);
    }

    #[test]
    fn landing_with_down_held_ducks() {
        let mut rex = Rex::new();
        rex.update(DT, PRESS);
        fly(&mut rex, DUCK);
        assert_eq!(rex.pose, Pose::Ducking);
    }

    #[test]
    fn ducks_while_down_is_held_and_stands_on_release() {
        let mut rex = Rex::new();
        rex.update(DT, DUCK);
        assert_eq!(rex.pose, Pose::Ducking);
        assert_eq!(rex.size(), DUCKING_SIZE);
        assert_eq!(rex.height, 0.0);

        rex.update(DT, NONE);
        assert_eq!(rex.pose, Pose::Running);
        assert_eq!(rex.size(), STANDING_SIZE);
    }

    #[test]
    fn time_after_touchdown_goes_to_running() {
        let mut rex = Rex {
            pose: Pose::Jumping,
            height: 1.0,
            velocity: -240.0,
            ..Rex::new()
        };
        rex.update(DT, JUMP);
        assert_eq!(rex.pose, Pose::Running);
        // Falling at 260 px/s from 1 px, it lands 1/260 s into the step, so the
        // remaining 1/120 - 1/260 s goes to the legs.
        let expected = DT - 1.0 / 260.0;
        assert!(
            (rex.step_timer - expected).abs() < 1e-5,
            "{}",
            rex.step_timer
        );
    }

    #[test]
    fn jump_wins_over_duck_on_the_ground() {
        let mut rex = Rex::new();
        rex.update(DT, DUCK);
        rex.update(
            DT,
            Input {
                jump_pressed: true,
                jump: true,
                duck: true,
            },
        );
        assert_eq!(rex.pose, Pose::Jumping);
    }
}
