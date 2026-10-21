//! Turns variable frame times into fixed physics ticks.
//!
//! Stepping physics by the raw frame time makes jump height depend on the
//! monitor's refresh rate, so every tick is the same length instead.

/// Length of one physics tick, matching the step the rex tests use.
pub const TICK: f32 = 1.0 / 120.0;
/// After a long stall (window dragged, laptop resumed) catching up on every
/// missed tick would freeze the game further, so the backlog is dropped.
const MAX_TICKS_PER_FRAME: u32 = 8;

#[derive(Debug, Default)]
pub struct Clock {
    pending: f32,
}

impl Clock {
    /// Adds a frame's elapsed time and returns how many ticks to run.
    pub fn advance(&mut self, frame_time: f32) -> u32 {
        self.pending += frame_time;
        let ticks = (self.pending / TICK) as u32;
        if ticks > MAX_TICKS_PER_FRAME {
            self.pending = 0.0;
            return MAX_TICKS_PER_FRAME;
        }
        self.pending -= ticks as f32 * TICK;
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_frames_accumulate_into_ticks() {
        let mut clock = Clock::default();
        let total: u32 = (0..144).map(|_| clock.advance(1.0 / 144.0)).sum();
        assert!((119..=120).contains(&total), "{total} ticks");
    }

    #[test]
    fn long_frames_run_several_ticks() {
        let mut clock = Clock::default();
        let total: u32 = (0..30).map(|_| clock.advance(1.0 / 30.0)).sum();
        assert!((119..=120).contains(&total), "{total} ticks");
    }

    #[test]
    fn a_stall_is_capped_and_forgotten() {
        let mut clock = Clock::default();
        assert_eq!(clock.advance(2.0), MAX_TICKS_PER_FRAME);
        assert_eq!(clock.advance(0.0), 0);
    }
}
