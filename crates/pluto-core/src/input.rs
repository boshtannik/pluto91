//! Button gesture recognition: turns raw "button is down" samples into
//! [`GestureEvent`]s (press / hold auto-repeat / double-click).
//!
//! A *press* is recognised on the falling edge of the button (its release):
//! a quick tap fires a `Press` (or a `Double` if a previous press happened
//! recently); holding the button past `HOLD_DELAY_MS` fires `Hold`
//! auto-repeats instead (driven by the repeated samples platforms send) and
//! suppresses the press.
//!
//! The same scanner drives both the real firmware (a poll loop sampling the
//! GPIO pins a few times per second) and the emulator (discrete `mousedown` /
//! `keydown` events), so the gesture timing is identical on both.

use crate::face::{ButtonId, GestureEvent, GestureKind};

/// How long a button must be held before auto-repeat starts.
pub const HOLD_DELAY_MS: u64 = 750;
/// How often a held button re-fires after `HOLD_DELAY_MS`.
pub const REPEAT_MS: u64 = 250;
/// A press within this many ms of the previous press counts as a double-click.
pub const DOUBLE_CLICK_MS: u64 = 400;

/// Per-button gesture state machine.
///
/// Feed it [`ButtonScanner::sample`] whenever the button's physical state is
/// known (on every poll on real hardware, or on down/up events in the
/// emulator). It returns the [`GestureKind`] of the transition, if any.
#[derive(Clone, Copy, Debug)]
pub struct ButtonScanner {
    /// The button is currently physically pressed.
    pressed: bool,
    /// ms of the current press (its down), for the hold delay and double
    /// detection.
    press_ms: u64,
    /// ms of the last `Hold` auto-repeat, for spacing repeats.
    last_repeat_ms: u64,
    /// ms of the previous press (its down), to detect a double-click.
    prev_press_ms: Option<u64>,
    /// A `Hold` already fired for the current press, so its release must not
    /// also fire a press.
    held: bool,
}

impl ButtonScanner {
    pub const fn new() -> Self {
        ButtonScanner {
            pressed: false,
            press_ms: 0,
            last_repeat_ms: 0,
            prev_press_ms: None,
            held: false,
        }
    }

    /// Forget the previous press, so the next tap of this button is a plain
    /// `Press`. Called by the runtime after a chord, so the buttons of a
    /// simultaneous pair do not ghost a `Double` for their next single tap.
    pub fn reset(&mut self) {
        self.pressed = false;
        self.held = false;
        self.prev_press_ms = None;
    }

    /// Sample the current physical state of the button.
    ///
    /// `down` is whether the button is pressed, `now_ms` the current time in
    /// ms (monotonic). Returns the gesture that this sample represents, or
    /// `None` if nothing changed / it's still inside the hold delay.
    pub fn sample(&mut self, down: bool, now_ms: u64) -> Option<GestureKind> {
        if down {
            if self.pressed {
                // Still held: auto-repeat after the hold delay.
                if now_ms.saturating_sub(self.press_ms) >= HOLD_DELAY_MS
                    && now_ms.saturating_sub(self.last_repeat_ms) >= REPEAT_MS
                {
                    self.last_repeat_ms = now_ms;
                    self.held = true;
                    return Some(GestureKind::Hold);
                }
                return None;
            }
            // Fresh press (rising edge): remember it; the press itself fires
            // when the button is released.
            self.pressed = true;
            self.press_ms = now_ms;
            self.last_repeat_ms = now_ms;
            None
        } else if self.pressed {
            // Release (falling edge).
            self.pressed = false;
            // The button was held and a `Hold` already fired: do not also
            // fire a press on release.
            if self.held {
                self.held = false;
                self.prev_press_ms = None;
                return None;
            }
            // A quick tap: a press within `DOUBLE_CLICK_MS` of the previous
            // one is a double-click. A press *before* the previous one (the
            // clock jumped backwards, e.g. the RTC was written back) is not.
            let is_double = self.prev_press_ms.is_some_and(|prev| {
                self.press_ms >= prev
                    && self.press_ms.saturating_sub(prev) <= DOUBLE_CLICK_MS
            });
            self.prev_press_ms = Some(self.press_ms);
            if is_double {
                Some(GestureKind::Double)
            } else {
                Some(GestureKind::Press)
            }
        } else {
            None
        }
    }

    /// Convenience: sample a button and wrap the result in a [`GestureEvent`].
    pub fn event(&mut self, button: ButtonId, down: bool, now_ms: u64) -> Option<GestureEvent> {
        self.sample(down, now_ms).map(|kind| GestureEvent { button, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press_at(s: &mut ButtonScanner, t: u64) -> Option<GestureKind> {
        s.sample(true, t)
    }

    fn release_at(s: &mut ButtonScanner, t: u64) -> Option<GestureKind> {
        s.sample(false, t)
    }

    fn tap(s: &mut ButtonScanner, down_t: u64, up_t: u64) -> Option<GestureKind> {
        press_at(s, down_t);
        release_at(s, up_t)
    }

    #[test]
    fn quick_tap_is_press_on_release() {
        let mut s = ButtonScanner::new();
        assert_eq!(press_at(&mut s, 1000), None);
        assert_eq!(release_at(&mut s, 1400), Some(GestureKind::Press));
    }

    #[test]
    fn no_repeat_before_hold_delay() {
        let mut s = ButtonScanner::new();
        press_at(&mut s, 0);
        assert_eq!(s.sample(true, HOLD_DELAY_MS - 1), None);
    }

    #[test]
    fn hold_autorepeats_every_repeat_ms() {
        let mut s = ButtonScanner::new();
        press_at(&mut s, 0);
        assert_eq!(s.sample(true, HOLD_DELAY_MS), Some(GestureKind::Hold));
        assert_eq!(
            s.sample(true, HOLD_DELAY_MS + REPEAT_MS),
            Some(GestureKind::Hold)
        );
        assert_eq!(
            s.sample(true, HOLD_DELAY_MS + REPEAT_MS + 1),
            None
        );
    }

    #[test]
    fn hold_suppresses_press_on_release() {
        let mut s = ButtonScanner::new();
        press_at(&mut s, 0);
        s.sample(true, HOLD_DELAY_MS);
        assert_eq!(release_at(&mut s, HOLD_DELAY_MS + 100), None);
        // ... and the next tap is a plain press, not a double.
        assert_eq!(tap(&mut s, HOLD_DELAY_MS + 500, HOLD_DELAY_MS + 600), Some(GestureKind::Press));
    }

    #[test]
    fn double_click_detected() {
        let mut s = ButtonScanner::new();
        assert_eq!(tap(&mut s, 0, 200), Some(GestureKind::Press));
        assert_eq!(tap(&mut s, 300, 400), Some(GestureKind::Double));
    }

    #[test]
    fn separate_presses_not_double() {
        let mut s = ButtonScanner::new();
        assert_eq!(tap(&mut s, 0, 200), Some(GestureKind::Press));
        assert_eq!(tap(&mut s, 600, 700), Some(GestureKind::Press));
    }

    #[test]
    fn reset_forgets_previous_press() {
        let mut s = ButtonScanner::new();
        assert_eq!(tap(&mut s, 0, 100), Some(GestureKind::Press));
        s.reset();
        // Right after reset the previous press is forgotten -> not a double.
        assert_eq!(tap(&mut s, 150, 250), Some(GestureKind::Press));
    }
}
