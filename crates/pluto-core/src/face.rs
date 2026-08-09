//! The `Face` trait: a "program" / mode of the watch.

use crate::hardware::Hardware;
use crate::time::DateTime;

/// The physical buttons. `Mode` is handled by the runtime (it cycles through
/// faces); `Light` and `Alarm` are passed through to the active face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId {
    Light,
    Mode,
    Alarm,
}

/// How a button event should be interpreted: a plain press, a held button
/// (auto-repeat), or a double-click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind {
    /// A fresh press (button went down).
    Press,
    /// The button is being held (auto-repeat, see `HOLD_DELAY_MS`).
    Hold,
    /// A second press within the double-click window of the previous press.
    Double,
}

/// A button gesture: which button and how it was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureEvent {
    pub button: ButtonId,
    pub kind: GestureKind,
}

impl GestureEvent {
    pub const fn press(button: ButtonId) -> Self {
        GestureEvent {
            button,
            kind: GestureKind::Press,
        }
    }

    pub const fn hold(button: ButtonId) -> Self {
        GestureEvent {
            button,
            kind: GestureKind::Hold,
        }
    }

    pub const fn double(button: ButtonId) -> Self {
        GestureEvent {
            button,
            kind: GestureKind::Double,
        }
    }
}

/// Two buttons pressed at the same time: `second` went down while `first` was
/// still held. Delivered to the active face (once both are released) instead
/// of the two separate presses, so faces can react to combinations (e.g.
/// Alarm + Light to re-seed an alarm with the current time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChordEvent {
    pub first: ButtonId,
    pub second: ButtonId,
}

impl ChordEvent {
    pub const fn new(first: ButtonId, second: ButtonId) -> Self {
        ChordEvent { first, second }
    }
}

/// Context handed to a face on every tick / button press.
#[derive(Clone, Copy, Debug)]
pub struct FaceContext {
    /// Current date and time, provided by the platform (emulator wall clock or
    /// the hardware RTC).
    pub time: DateTime,
    /// Whether the watch displays time in 24-hour (`true`) or 12-hour
    /// (`false`) format. Global state, toggled by the Alarm button in the
    /// runtime, so every face (and future faces) shows the same format.
    pub h24: bool,
    /// Whether the hourly chime (the "signal") is enabled. Global state,
    /// toggled by the Alarm button on the alarm face's view mode.
    pub chime: bool,
}

impl FaceContext {
    pub fn new(time: DateTime, h24: bool, chime: bool) -> Self {
        FaceContext { time, h24, chime }
    }
}

/// What the global Alarm-button handling does when the active face does not
/// consume the press. The alarm face uses the button to toggle the hourly
/// chime; every other face toggles the 12/24-hour format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlarmAction {
    /// Toggle the global 12/24-hour format.
    H24Toggle,
    /// Toggle the global hourly chime.
    ChimeToggle,
}

/// A face (program) of the watch.
///
/// Implementations should draw the whole visible state on every `tick`
/// (segment writes are idempotent, so re-drawing is cheap and safe) and use
/// the [`Hardware`] handle for non-display effects (backlight, buzzer).
pub trait Face {
    /// Called once when the face becomes active (after boot or after the
    /// user pressed Mode). Resets any per-face state and draws the initial
    /// frame.
    fn init(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) {}
    /// Called periodically (once per second or more often) for the **active**
    /// face only.
    fn tick(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) {}
    /// Called periodically for **every** face, active or not. Use for
    /// background behaviour that must run regardless of the visible face,
    /// e.g. detecting that an alarm time was reached. Do not draw here.
    ///
    /// Return `true` to request becoming the active face: the runtime then
    /// switches to this face right away (first requester in [`FaceSet`] order
    /// wins). The active face's own request is ignored. The face must leave
    /// itself ready to be shown, since the runtime skips [`Self::init`] on an
    /// auto-switch (it is already drawn correctly by the tick that follows).
    /// Defaults to `false`.
    fn background_tick(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }
    /// Called when a button is pressed. `event` describes the gesture:
    /// a plain press, a held button (auto-repeat), or a double-click.
    ///
    /// Only the **active** face receives button events, so anything that
    /// fires only while the face is shown (e.g. a ringing alarm) can simply
    /// be silenced here on any button.
    ///
    /// Returns `true` if the event was fully handled by the face. The
    /// runtime then skips its global handling of that button (e.g. the Alarm
    /// button toggling the 12/24h format). Return `false` to let the global
    /// behaviour also run (e.g. `SimpleClock` lets Alarm toggle the format
    /// while it just beeps).
    fn button(&mut self, _event: GestureEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    /// Called when two buttons are pressed at the same time (see
    /// [`ChordEvent`]). Faces that care about combinations override this;
    /// others get the default no-op and simply never see the individual
    /// presses of that pair. Returns `true` if the chord was fully handled
    /// (same contract as [`Self::button`]).
    fn chord(&mut self, _event: ChordEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    /// Which global action the Alarm button triggers when this face does not
    /// consume the press. Defaults to the 12/24-hour format; the alarm face
    /// overrides it to toggle the hourly chime instead.
    fn alarm_action(&self) -> AlarmAction {
        AlarmAction::H24Toggle
    }
}
