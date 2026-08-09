//! Casio-style countdown timer face (the stock F-91W ST mode).
//!
//! The view mode shows `TI` in the weekday letters and the remaining time as
//! HH:MM:SS (no leading zero on the hour, like the clock face). The **LAP**
//! indicator is on while the countdown runs. The Alarm button starts / pauses
//! it; a finished countdown (stopped at 00:00:00) restarts from the full
//! duration. On the very first entry the timer is pre-set to the first preset
//! (1 minute), so the face never greets you with a useless 00:00:00.
//!
//! Light enters the settings: the fields are seconds -> minutes -> hours (Light
//! advances, Alarm steps the value, a double adds 5, a hold resets to 0), like
//! the alarm faces. Entering the settings pauses any running countdown, and
//! leaving them arms the freshly configured duration (reset to its full length,
//! paused). The maximum duration is 23:59:59.
//!
//! The Alarm + Light chord steps through the preset durations
//! (1/3/5/7/10/15/20/30/40/60 minutes): it sets the duration and stops /
//! resets the current countdown.
//!
//! The countdown runs in the background (via `background_tick`) on any face.
//! When it reaches zero the watch auto-switches here and rings for up to
//! [`RING_SECS`] (or until any button press), showing 00:00:00 with the Bell
//! indicator blinking at 2 Hz.

use pluto_core::face::{ButtonId, ChordEvent, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::font::Indicator;
use pluto_core::{DigitDisplay, Hardware};

/// How long a finished countdown keeps ringing (seconds), unless silenced by a
/// button press first. Same as the alarm faces.
const RING_SECS: u32 = 120;

/// Preset durations in minutes, cycled by the Alarm + Light chord.
const PRESETS_MIN: [u32; 10] = [1, 3, 5, 7, 10, 15, 20, 30, 40, 60];

/// After a change of the edited value the focus is shown steadily (no
/// blinking) for this long, so fast scrolling via the hold auto-repeat or a
/// double press stays readable.
const NO_BLINK_MS: u64 = 750;

/// Which value is being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerField {
    Sec,
    Min,
    Hour,
}

/// Display mode of the face.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    View,
    Edit(TimerField),
}

/// The countdown timer face.
#[derive(Clone, Copy, PartialEq)]
pub struct Timer {
    /// Configured countdown length in ms.
    duration_ms: u64,
    /// Milliseconds left until zero (never more than `duration_ms`).
    remaining_ms: u64,
    /// Whether the countdown is currently running.
    running: bool,
    /// Epoch ms of the last background tick while running, to compute the
    /// elapsed wall-clock time. Stale when paused (and re-armed on resume).
    last_tick_ms: u64,
    /// Index into [`PRESETS_MIN`] of the currently selected preset.
    preset: usize,
    mode: TimerMode,
    /// Epoch ms of the last change of the edited value, to suppress blinking
    /// while the user is rapidly scrolling. `None` when unchanged.
    changed_at: Option<u64>,
    /// Epoch second at which the ring ends. `None` when not ringing.
    ring_until: Option<u64>,
}

impl Timer {
    pub const fn new() -> Self {
        let duration_ms = PRESETS_MIN[0] as u64 * 60_000;
        Timer {
            duration_ms,
            remaining_ms: duration_ms,
            running: false,
            last_tick_ms: 0,
            preset: 0,
            mode: TimerMode::View,
            changed_at: None,
            ring_until: None,
        }
    }

    /// The configured duration, in whole seconds.
    pub fn duration_secs(&self) -> u32 {
        (self.duration_ms / 1000) as u32
    }

    /// The configured duration split into (hour, minute, second).
    fn duration_parts(&self) -> (u8, u8, u8) {
        let total = (self.duration_ms / 1000) as u32;
        (
            ((total / 3600) % 24) as u8,
            ((total / 60) % 60) as u8,
            (total % 60) as u8,
        )
    }

    /// Rebuild `duration_ms` from (hour, minute, second).
    fn set_duration_parts(&mut self, hour: u8, min: u8, sec: u8) {
        self.duration_ms = (hour as u64 * 3600 + min as u64 * 60 + sec as u64) * 1000;
    }

    fn step(&mut self, steps: u8) {
        let (mut hour, mut min, mut sec) = self.duration_parts();
        match self.mode {
            TimerMode::Edit(TimerField::Sec) => sec = (sec + steps) % 60,
            TimerMode::Edit(TimerField::Min) => min = (min + steps) % 60,
            TimerMode::Edit(TimerField::Hour) => hour = (hour + steps) % 24,
            TimerMode::View => return,
        }
        self.set_duration_parts(hour, min, sec);
    }

    /// Reset the focused value to zero (holding the Alarm button).
    fn reset(&mut self) {
        let (mut hour, mut min, mut sec) = self.duration_parts();
        match self.mode {
            TimerMode::Edit(TimerField::Sec) => sec = 0,
            TimerMode::Edit(TimerField::Min) => min = 0,
            TimerMode::Edit(TimerField::Hour) => hour = 0,
            TimerMode::View => return,
        }
        self.set_duration_parts(hour, min, sec);
    }

    /// Step to the next preset: set the duration, reset the countdown and stop
    /// any run.
    fn next_preset(&mut self) {
        self.preset = (self.preset + 1) % PRESETS_MIN.len();
        self.duration_ms = PRESETS_MIN[self.preset] as u64 * 60_000;
        self.remaining_ms = self.duration_ms;
        self.running = false;
    }

    /// The remaining time rounded up to whole seconds, as (hour, min, sec).
    /// Ceiling keeps a freshly started timer showing its full seconds until a
    /// full second has actually elapsed.
    fn remaining_parts(&self) -> (u8, u8, u8) {
        let secs = (self.remaining_ms + 999) / 1000;
        ((secs / 3600) as u8, ((secs / 60) % 60) as u8, (secs % 60) as u8)
    }

    fn draw_pair(&self, hw: &mut impl Hardware, position: u8, value: u8, visible: bool) {
        if visible {
            hw.set_digit(position, value / 10);
            hw.set_digit(position + 1, value % 10);
        } else {
            hw.clear_digit(position);
            hw.clear_digit(position + 1);
        }
    }

    /// Draw HH:MM:SS into the time digits, no leading zero on the hour (like
    /// the clock face).
    fn draw_clock(&self, hw: &mut impl Hardware, hour: u8, min: u8, sec: u8) {
        if hour >= 10 {
            hw.set_digit(4, hour / 10);
        } else {
            hw.clear_digit(4);
        }
        hw.set_digit(5, hour % 10);
        hw.set_digit(6, min / 10);
        hw.set_digit(7, min % 10);
        hw.set_digit(8, sec / 10);
        hw.set_digit(9, sec % 10);
    }

    fn draw_view(&self, ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.set_char(0, b'T');
        hw.set_char(1, b'I');
        let (hour, min, sec) = self.remaining_parts();
        self.draw_clock(hw, hour, min, sec);
        // The LAP indicator means "the countdown is running". The Bell
        // indicator blinks at 2 Hz while the ring fires; otherwise it is off.
        let ringing = self.ring_until.is_some();
        let bell = ringing && (ctx.time.ms / 500) % 2 == 0;
        hw.set_indicator(Indicator::Lap, self.running);
        hw.set_indicator(Indicator::Bell, bell);
        hw.set_indicator(Indicator::H24, false);
        hw.set_indicator(Indicator::Pm, false);
        hw.set_indicator(Indicator::Signal, ctx.chime);
    }

    fn draw_edit(&self, t: &pluto_core::time::DateTime, hw: &mut impl Hardware) {
        // While the user is rapidly stepping through values (hold auto-repeat
        // or a double press) the focused value is shown steadily instead of
        // blinking; blinking resumes shortly after the last change.
        let now_ms = t.secs as u64 * 1000 + t.ms as u64;
        let recently_changed = self
            .changed_at
            .is_some_and(|c| now_ms.saturating_sub(c) < NO_BLINK_MS);
        let blink = recently_changed || (t.ms / 250) % 2 == 0;
        let field = match self.mode {
            TimerMode::Edit(f) => f,
            TimerMode::View => return,
        };

        match field {
            TimerField::Sec => {
                hw.set_char(0, b'S');
                hw.set_char(1, b'E');
            }
            TimerField::Min => {
                hw.set_char(0, b'M');
                hw.set_char(1, b'I');
            }
            TimerField::Hour => {
                hw.set_char(0, b'H');
                hw.set_char(1, b'O');
            }
        }

        let (hour, min, sec) = self.duration_parts();
        // The edited hours always show their tens, so the maximum 23:59:59 is
        // unambiguous while configuring.
        hw.set_digit(4, hour / 10);
        hw.set_digit(5, hour % 10);
        self.draw_pair(hw, 6, min, field != TimerField::Min || blink);
        self.draw_pair(hw, 8, sec, field != TimerField::Sec || blink);

        hw.set_indicator(Indicator::Lap, false);
        hw.set_indicator(Indicator::Bell, false);
        hw.set_indicator(Indicator::H24, false);
        hw.set_indicator(Indicator::Pm, false);
        hw.set_indicator(Indicator::Signal, false);
    }
}

impl Face for Timer {
    fn init(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) {
        // Entering the face always starts in the view mode and silences the
        // ring (a Mode exit from the middle of a ring or an edit session must
        // not resume either). An auto-switch when the countdown ends does NOT
        // call init, so the ring survives that path. The countdown itself
        // keeps running: coming back from another face resumes it. An edit
        // session abandoned with Mode re-arms the full duration.
        let was_editing = matches!(self.mode, TimerMode::Edit(_));
        self.mode = TimerMode::View;
        self.ring_until = None;
        if was_editing && !self.running {
            self.remaining_ms = self.duration_ms;
        }
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        // Ring while inside the window; go silent once we pass it. Only the
        // active face ticks, so a ring dies the moment the user switches faces
        // with Mode.
        match self.ring_until {
            Some(until) if ctx.time.secs < until => hw.beep(),
            Some(_) => self.ring_until = None,
            None => {}
        }
        match self.mode {
            TimerMode::View => self.draw_view(ctx, hw),
            TimerMode::Edit(_) => self.draw_edit(&ctx.time, hw),
        }
    }

    fn background_tick(&mut self, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        let now_ms = ctx.time.secs as u64 * 1000 + ctx.time.ms as u64;
        if self.running {
            let elapsed = now_ms.saturating_sub(self.last_tick_ms);
            self.last_tick_ms = now_ms;
            self.remaining_ms = self.remaining_ms.saturating_sub(elapsed);
            if self.remaining_ms == 0 {
                self.running = false;
                // Snap to the view mode: the countdown ending interrupts
                // whatever the face was doing (even its own edit session).
                self.mode = TimerMode::View;
                self.ring_until = Some(ctx.time.secs + RING_SECS as u64);
                return true;
            }
        }
        false
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        // Any button press silences the ring (the stock Casio behaviour).
        self.ring_until = None;
        let now_ms = ctx.time.secs as u64 * 1000 + ctx.time.ms as u64;
        match event.button {
            ButtonId::Alarm => match self.mode {
                // In the view mode Alarm starts / pauses the countdown; a
                // finished countdown (stopped at 00:00:00) restarts from the
                // full duration.
                TimerMode::View => {
                    if event.kind == GestureKind::Press {
                        if self.running {
                            self.running = false;
                        } else {
                            if self.remaining_ms == 0 {
                                self.remaining_ms = self.duration_ms;
                            }
                            self.running = true;
                            self.last_tick_ms = now_ms;
                        }
                    }
                }
                TimerMode::Edit(_) => match event.kind {
                    // A plain press adds one unit.
                    GestureKind::Press => {
                        self.step(1);
                        self.changed_at = Some(now_ms);
                    }
                    // Holding the button resets the value to zero.
                    GestureKind::Hold => {
                        self.reset();
                        self.changed_at = Some(now_ms);
                    }
                    // A double press adds five units in total: the first press
                    // of the pair already added one, so the second adds four
                    // more.
                    GestureKind::Double => {
                        self.step(4);
                        self.changed_at = Some(now_ms);
                    }
                },
            },
            ButtonId::Light => match self.mode {
                // A press enters the settings (no hold needed). The countdown
                // is paused: the face shows a duration, not a ticking
                // remaining time, while editing.
                TimerMode::View => {
                    if event.kind == GestureKind::Press {
                        self.running = false;
                        self.mode = TimerMode::Edit(TimerField::Sec);
                    }
                }
                TimerMode::Edit(field) => {
                    match event.kind {
                        // A press moves to the next field; on the last one
                        // (hour) it exits the settings.
                        GestureKind::Press => {
                            self.mode = match field {
                                TimerField::Sec => TimerMode::Edit(TimerField::Min),
                                TimerField::Min => TimerMode::Edit(TimerField::Hour),
                                TimerField::Hour => TimerMode::View,
                            };
                        }
                        // A hold exits the settings.
                        GestureKind::Hold => self.mode = TimerMode::View,
                        _ => {}
                    }
                    // Exiting the settings arms the freshly configured
                    // duration: the countdown resets to its full length,
                    // paused, ready to start on Alarm.
                    if self.mode == TimerMode::View {
                        self.remaining_ms = self.duration_ms;
                    }
                }
            },
            _ => return false,
        }
        true
    }

    fn chord(&mut self, event: ChordEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        // Only the Alarm + Light combination does anything here (in either
        // press order): it cycles to the next preset duration.
        let both = [event.first, event.second];
        if !both.contains(&ButtonId::Alarm) || !both.contains(&ButtonId::Light) {
            return false;
        }
        self.next_preset();
        true
    }
}
