//! Game controllers, read through gilrs. Which button does what is decided
//! apart from gilrs's event loop, so it can be tested without a controller.

use gilrs::{Axis, Button, EventType, Gilrs};

/// How far the left stick has to tilt before it counts, so a stick resting
/// slightly off centre doesn't jump or duck on its own.
const STICK_THRESHOLD: f32 = 0.5;

/// Every button `action` maps, to check which are held.
const MAPPED: [Button; 4] = [
    Button::South,
    Button::DPadUp,
    Button::DPadDown,
    Button::Start,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Jump,
    Duck,
    Pause,
}

/// A (Xbox) or cross (PlayStation) and d-pad up jump, d-pad down ducks and
/// Start pauses, matching Space, Up, Down and P on the keyboard.
pub fn action(button: Button) -> Option<Action> {
    match button {
        Button::South | Button::DPadUp => Some(Action::Jump),
        Button::DPadDown => Some(Action::Duck),
        Button::Start => Some(Action::Pause),
        _ => None,
    }
}

/// The left stick works like the d-pad. gilrs gives y as positive upwards.
pub fn stick_action(y: f32) -> Option<Action> {
    if y > STICK_THRESHOLD {
        Some(Action::Jump)
    } else if y < -STICK_THRESHOLD {
        Some(Action::Duck)
    } else {
        None
    }
}

/// What every connected controller asked for since the last poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadInput {
    pub jump: bool,
    pub duck: bool,
    /// Pressed since the last poll, even if already released again.
    pub jump_pressed: bool,
    pub pause_pressed: bool,
}

impl PadInput {
    fn hold(&mut self, action: Option<Action>) {
        match action {
            Some(Action::Jump) => self.jump = true,
            Some(Action::Duck) => self.duck = true,
            Some(Action::Pause) | None => {}
        }
    }

    fn press(&mut self, action: Option<Action>) {
        match action {
            Some(Action::Jump) => self.jump_pressed = true,
            Some(Action::Pause) => self.pause_pressed = true,
            Some(Action::Duck) | None => {}
        }
    }
}

pub struct Pads {
    /// None when controllers can't be read at all; the keyboard still works.
    gilrs: Option<Gilrs>,
}

impl Pads {
    pub fn new() -> Self {
        let gilrs = Gilrs::new()
            .inspect_err(|e| eprintln!("rusty-rex: no controller support: {e}"))
            .ok();
        Self { gilrs }
    }

    pub fn poll(&mut self) -> PadInput {
        let mut input = PadInput::default();
        let Some(gilrs) = &mut self.gilrs else {
            return input;
        };
        // Draining the events also keeps gilrs's view of each pad current.
        while let Some(event) = gilrs.next_event() {
            if let EventType::ButtonPressed(button, _) = event.event {
                input.press(action(button));
            }
        }
        for (_, pad) in gilrs.gamepads() {
            for button in MAPPED.into_iter().filter(|&b| pad.is_pressed(b)) {
                input.hold(action(button));
            }
            input.hold(stick_action(pad.value(Axis::LeftStickY)));
        }
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_match_the_keyboard() {
        assert_eq!(action(Button::South), Some(Action::Jump));
        assert_eq!(action(Button::DPadUp), Some(Action::Jump));
        assert_eq!(action(Button::DPadDown), Some(Action::Duck));
        assert_eq!(action(Button::Start), Some(Action::Pause));
        assert_eq!(action(Button::West), None);
    }

    #[test]
    fn every_mapped_button_does_something() {
        assert!(MAPPED.iter().all(|&b| action(b).is_some()));
    }

    #[test]
    fn the_stick_needs_a_real_tilt() {
        assert_eq!(stick_action(0.0), None);
        assert_eq!(stick_action(0.3), None);
        assert_eq!(stick_action(-0.3), None);
        assert_eq!(stick_action(0.9), Some(Action::Jump));
        assert_eq!(stick_action(-0.9), Some(Action::Duck));
    }

    #[test]
    fn holding_and_pressing_set_separate_flags() {
        let mut input = PadInput::default();
        input.hold(Some(Action::Duck));
        input.press(Some(Action::Jump));
        input.press(Some(Action::Pause));
        assert_eq!(
            input,
            PadInput {
                jump: false,
                duck: true,
                jump_pressed: true,
                pause_pressed: true,
            }
        );
    }
}
