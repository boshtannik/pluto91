//! Button gesture recognition. Turns raw "button is down" samples into the
//! pluto key events (`Up` / `Down` / `Enter`, plain or `Long`).
//!
//! The same scanner drives the real firmware (a poll loop sampling the GPIO
//! pins) and the emulator (discrete down/up events), so gesture timing is
//! identical on both.

use crate::app::Event;

/// How long a button must be held before it becomes a `*Long` event / starts
/// auto-repeating.
pub const HOLD_DELAY_MS: u64 = 750;
/// How often a held button re-fires after `HOLD_DELAY_MS`.
pub const REPEAT_MS: u64 = 250;

/// The pluto buttons, by their logical role (same physical buttons as the
/// F-91W: Mode = Enter, Light = Down, Alarm = Up).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyId {
    Up,
    Down,
    Enter,
}

/// Per-button gesture state machine.
///
/// Feed it [`ButtonScanner::sample`] whenever the button's physical state is
/// known. Returns the resulting [`Event`], or `None` if nothing happened.
#[derive(Clone, Copy, Debug)]
pub struct ButtonScanner {
    pressed: bool,
    press_ms: u64,
    last_repeat_ms: u64,
}

impl ButtonScanner {
    pub const fn new() -> Self {
        ButtonScanner {
            pressed: false,
            press_ms: 0,
            last_repeat_ms: 0,
        }
    }

    /// Sample the current physical state of the button.
    pub fn sample(&mut self, key: KeyId, down: bool, now_ms: u64) -> Option<Event> {
        if down {
            if self.pressed {
                if now_ms.saturating_sub(self.press_ms) >= HOLD_DELAY_MS
                    && now_ms.saturating_sub(self.last_repeat_ms) >= REPEAT_MS
                {
                    self.last_repeat_ms = now_ms;
                    return Some(Event::key_long(key));
                }
                return None;
            }
            self.pressed = true;
            self.press_ms = now_ms;
            self.last_repeat_ms = now_ms;
            Some(Event::key(key))
        } else {
            self.pressed = false;
            None
        }
    }
}
