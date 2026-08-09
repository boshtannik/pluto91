//! The runtime: owns the active face, the backlight timer, and dispatches
//! ticks and buttons. Raw button samples go through [`Watch::button_raw`],
//! which turns them into gestures (press / hold auto-repeat / double-click)
//! and detects simultaneous presses (chords).

use crate::face::{AlarmAction, ButtonId, ChordEvent, Face, FaceContext, GestureEvent, GestureKind};
use crate::hardware::Hardware;
use crate::input::ButtonScanner;
use crate::time::DateTime;
/// A face type (usually an enum of all faces) together with the ability to
/// enumerate and switch between faces.
pub trait FaceSet: Face + Copy + Default + 'static {
    /// Number of faces.
    const LEN: usize;
    /// A fixed-size container holding one instance of every face (e.g.
    /// `[Faces; 2]`), used by the runtime to keep per-face state alive
    /// across mode changes.
    type Faces: Copy + AsRef<[Self]> + AsMut<[Self]>;
    /// All faces in order, used for Mode cycling.
    fn all() -> &'static [Self];
    /// Build the face store, one instance of every face.
    fn faces() -> Self::Faces;
}

/// How long the backlight stays on after the Light button is pressed.
const BACKLIGHT_SECS: u64 = 3;

/// The physical buttons, indexed like the per-button gesture scanners
/// (`ButtonId` is `Light = 0, Mode = 1, Alarm = 2`).
const BUTTON_IDS: [ButtonId; 3] = [ButtonId::Light, ButtonId::Mode, ButtonId::Alarm];

/// Owns the faces and dispatches events to the active one.
///
/// Every face lives in [`Watch`] permanently: switching modes with the Mode
/// button swaps the active face but keeps each face's state, so data like
/// the alarm settings survive mode changes.
#[derive(Clone, Copy, Debug)]
pub struct Watch<F: FaceSet> {
    /// One instance of every face. `face_index` selects the active one.
    faces: F::Faces,
    /// Index of the active face, for Mode cycling.
    face_index: usize,
    /// Epoch second at which the backlight should be turned off.
    backlight_until: Option<u64>,
    /// Time format: `true` = 24-hour, `false` = 12-hour. Toggled by the
    /// Alarm button and shared with the active face via [`FaceContext`].
    h24: bool,
    /// Hourly chime (the "signal"): beeps at the top of every hour. Toggled
    /// by the Alarm button on the alarm face's view mode.
    chime: bool,
    /// Epoch hour of the last chime, so the top-of-hour beep fires once.
    last_chime: Option<u64>,
    /// Per-button gesture scanners (press on release / hold auto-repeat /
    /// double-click).
    scanners: [ButtonScanner; 3],
    /// Whether each button is currently physically pressed, so a chord (two
    /// buttons down at once) can be detected.
    pressed: [bool; 3],
    /// Whether the press of the current press-hold of each button has already
    /// been delivered (on release or on the first platform repeat while
    /// held). Used to suppress a press when a chord forms, and to avoid a
    /// ghost press on release after a delivered hold.
    press_delivered: [bool; 3],
    /// The chord that is forming: `(first, second)` where `second` went down
    /// while `first` was still held. Delivered once both are released.
    pending_chord: Option<(ButtonId, ButtonId)>,
}

impl<F: FaceSet> Watch<F> {
    pub fn new() -> Self {
        Watch {
            faces: F::faces(),
            face_index: 0,
            backlight_until: None,
            h24: true,
            chime: false,
            last_chime: None,
            scanners: [ButtonScanner::new(); 3],
            pressed: [false; 3],
            press_delivered: [false; 3],
            pending_chord: None,
        }
    }

    pub fn active_face(&self) -> F {
        self.faces.as_ref()[self.face_index]
    }

    /// Periodic tick. `time` comes from the platform (RTC or emulator clock).
    pub fn tick(&mut self, time: DateTime, hw: &mut impl Hardware) {
        let ctx = FaceContext::new(time, self.h24, self.chime);
        if let Some(until) = self.backlight_until {
            if time.secs >= until {
                self.backlight_until = None;
                hw.set_backlight(false);
            }
        }
        // Hourly chime: beep once at the top of every hour when enabled.
        if self.chime && time.minute == 0 && time.second == 0 {
            let hour = time.secs / 3600;
            if self.last_chime != Some(hour) {
                self.last_chime = Some(hour);
                hw.beep();
            }
        }
        // Background tick for every face (alarms fire even when the clock
        // face is shown). A face may request to become active; the first one
        // in FaceSet order wins and the watch switches to it (skipping init:
        // the face left itself ready to be shown, so its tick just redraws).
        let mut activate: Option<usize> = None;
        for (i, f) in self.faces.as_mut().iter_mut().enumerate() {
            let want = f.background_tick(&ctx, hw);
            if i != self.face_index && want {
                activate.get_or_insert(i);
            }
        }
        if let Some(i) = activate {
            self.face_index = i;
            hw.clear_all();
        }
        self.faces.as_mut()[self.face_index].tick(&ctx, hw);
    }

    /// Raw button sample. Called on every state change: `down` is whether
    /// the button is currently pressed, `time` the wall clock. The runtime
    /// classifies the samples: a quick tap fires a `Press` on the button's
    /// release, two quick taps a `Double`, and holding past the hold delay
    /// fires `Hold` auto-repeats. The *first* auto-repeat of a hold also acts
    /// as the press of the hold, so a held button reacts while it is still
    /// pressed (holding Mode cycles faces, holding Light turns on the
    /// backlight) instead of only on its release. When a second button goes
    /// down while another one is still held, the two form a chord: once both
    /// are released the pair is delivered to the active face as a
    /// [`ChordEvent`] instead of the two separate presses, and the press and
    /// hold auto-repeat of the chord's buttons are suppressed (so a held
    /// button in a chord does not fire its action early).
    ///
    /// The platforms call this from their poll loop (real hardware) or from
    /// the button events (emulator); the timing is identical on both.
    pub fn button_raw(&mut self, id: ButtonId, down: bool, time: DateTime, hw: &mut impl Hardware) {
        let ctx = FaceContext::new(time, self.h24, self.chime);
        let now = now_ms(time);
        let idx = id as usize;
        if down {
            if !self.pressed[idx] {
                // A fresh press: clear the per-press flag, then check whether
                // another button is already held (a chord in the making). The
                // chord's buttons never fire their own press or hold; the pair
                // is delivered on release instead.
                self.press_delivered[idx] = false;
                for (i, other) in BUTTON_IDS.iter().enumerate() {
                    if *other != id && self.pressed[i] {
                        self.pending_chord = Some((*other, id));
                        self.press_delivered[idx] = true;
                        break;
                    }
                }
            }
            self.pressed[idx] = true;
            if let Some(event) = self.scanners[idx].event(id, true, now) {
                // The button is being held (auto-repeat past the hold delay).
                if let Some((first, second)) = self.pending_chord {
                    if first == id || second == id {
                        // Part of a chord: drop both the press and the hold;
                        // the chord fires when both buttons are released.
                        self.press_delivered[idx] = true;
                    }
                } else {
                    // Hybrid: the first auto-repeat also delivers the press of
                    // the hold (so a held button reacts right away), then the
                    // hold itself fires (backlight, alarm reset).
                    if !self.press_delivered[idx] {
                        self.press_delivered[idx] = true;
                        self.dispatch(GestureEvent::press(id), &ctx, hw);
                    }
                    self.dispatch(event, &ctx, hw);
                }
            }
        } else if let Some((first, second)) = self.pending_chord {
            // Release of a chord's button: wait until both are released, then
            // deliver the chord (and drop the scanner state of both buttons,
            // so a next tap is not ghosted as a double-click).
            self.pressed[idx] = false;
            self.scanners[first as usize].reset();
            self.scanners[second as usize].reset();
            if !self.pressed[first as usize] && !self.pressed[second as usize] {
                self.pending_chord = None;
                self.faces.as_mut()[self.face_index].chord(ChordEvent::new(first, second), &ctx, hw);
            }
        } else {
            self.pressed[idx] = false;
            // The scanner suppresses the release of a button whose press was
            // already delivered (as a chord or as the first auto-repeat of a
            // hold), so a release only ever fires a press for a quick tap.
            if let Some(event) = self.scanners[idx].event(id, false, now) {
                self.dispatch(event, &ctx, hw);
            }
        }
    }

    fn dispatch(&mut self, event: GestureEvent, ctx: &FaceContext, hw: &mut impl Hardware) {
        match event.button {
            ButtonId::Mode => {
                if event.kind == GestureKind::Press {
                    self.cycle_face(ctx, hw);
                }
            }
            ButtonId::Light => {
                if event.kind == GestureKind::Hold {
                    hw.set_backlight(true);
                    self.backlight_until = Some(ctx.time.secs + BACKLIGHT_SECS);
                }
                self.faces.as_mut()[self.face_index].button(event, ctx, hw);
            }
            ButtonId::Alarm => {
                // The face gets the event first; if it does not consume it, a
                // plain press triggers the face's global Alarm action
                // (12/24-hour format, or the hourly chime on the alarm face).
                let action = self.faces.as_mut()[self.face_index].alarm_action();
                let consumed = self.faces.as_mut()[self.face_index].button(event, ctx, hw);
                if event.kind == GestureKind::Press && !consumed {
                    match action {
                        AlarmAction::H24Toggle => self.h24 = !self.h24,
                        AlarmAction::ChimeToggle => self.chime = !self.chime,
                    }
                }
            }
        }
    }

    fn cycle_face(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        self.face_index = (self.face_index + 1) % F::LEN;
        hw.clear_all();
        self.faces.as_mut()[self.face_index].init(ctx, hw);
    }
}

fn now_ms(time: DateTime) -> u64 {
    time.secs as u64 * 1000 + time.ms as u64
}

impl<F: FaceSet> Default for Watch<F> {
    fn default() -> Self {
        Self::new()
    }
}
