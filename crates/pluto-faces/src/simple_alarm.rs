//! Simple Casio-style alarm face: one alarm, no weekdays.
//!
//! The view mode shows `AL` and the alarm time (in the current 12/24-hour
//! format, without seconds); while the alarm rings it shows the current time
//! in full (with seconds) instead and blinks the Bell indicator at 2 Hz, so
//! the display never mixes a stale alarm time with live seconds. The Alarm
//! button toggles the alarm on
//! and off (the Bell indicator), like the stock F-91W. Light enters the
//! settings: the fields are hour -> minute (Light advances, Alarm steps the
//! value). A fresh alarm (00:00, off) starts from the current time rounded up
//! to the next minute; the Alarm + Light chord re-seeds it with the current
//! time at any point. Any button press silences a ringing alarm.

use pluto_core::face::{ButtonId, ChordEvent, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::font::Indicator;
use pluto_core::{DigitDisplay, Hardware};

/// How long a fired alarm keeps ringing (seconds), unless silenced by a
/// button press first.
const RING_SECS: u32 = 120;

/// After a change of the edited value the focus is shown steadily (no
/// blinking) for this long, so fast scrolling via the hold auto-repeat or a
/// double press stays readable.
const NO_BLINK_MS: u64 = 750;

/// Convert a 0..23 hour into 12-hour form (0 -> 12, 13..=23 -> 1..=11).
const fn to_12h(hour: u8) -> u8 {
    match hour {
        0 => 12,
        13..=23 => hour - 12,
        _ => hour,
    }
}

/// Which value is being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SimpleAlarmField {
    Hour,
    Minute,
}

/// Display mode of the face.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SimpleAlarmMode {
    View,
    Edit(SimpleAlarmField),
}

/// The simple alarm face.
#[derive(Clone, Copy, PartialEq)]
pub struct SimpleAlarm {
    enabled: bool,
    hour: u8,
    minute: u8,
    mode: SimpleAlarmMode,
    /// Epoch ms of the last change of the edited value, to suppress blinking
    /// while the user is rapidly scrolling. `None` when unchanged.
    changed_at: Option<u64>,
    /// Last alarm that fired `(hour, minute)`, to fire only once per minute.
    last_fired: Option<(u8, u8)>,
    /// Seconds-of-day at which the ring ends. `None` when not ringing.
    ring_until: Option<u32>,
}

impl SimpleAlarm {
    pub const fn new() -> Self {
        SimpleAlarm {
            enabled: false,
            hour: 0,
            minute: 0,
            mode: SimpleAlarmMode::View,
            changed_at: None,
            last_fired: None,
            ring_until: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn time(&self) -> (u8, u8) {
        (self.hour, self.minute)
    }

    /// Seed the alarm with the current wall-clock time as the base value for
    /// editing, but only if it was never configured (disabled, 00:00): a
    /// configured alarm keeps its saved time. The user can force a re-seed
    /// with the current time via the Alarm + Light chord. Seconds are dropped:
    /// the alarm fires at the top of the minute, never in the middle of one.
    fn prime_from_now(&mut self, t: &pluto_core::time::DateTime) {
        if !self.enabled && self.hour == 0 && self.minute == 0 {
            self.hour = t.hour;
            self.minute = t.minute;
        }
    }

    /// Unconditionally set the alarm to the current wall-clock time (the
    /// Alarm + Light chord). Seconds are dropped, see
    /// [`Self::prime_from_now`].
    fn seed_now(&mut self, t: &pluto_core::time::DateTime) {
        self.hour = t.hour;
        self.minute = t.minute;
    }

    fn step(&mut self, steps: u8) {
        match self.mode {
            SimpleAlarmMode::Edit(SimpleAlarmField::Hour) => {
                self.hour = (self.hour + steps) % 24;
            }
            SimpleAlarmMode::Edit(SimpleAlarmField::Minute) => {
                self.minute = (self.minute + steps) % 60;
            }
            SimpleAlarmMode::View => {}
        }
    }

    /// Reset the focused value to zero (holding the Alarm button).
    fn reset(&mut self) {
        match self.mode {
            SimpleAlarmMode::Edit(SimpleAlarmField::Hour) => self.hour = 0,
            SimpleAlarmMode::Edit(SimpleAlarmField::Minute) => self.minute = 0,
            SimpleAlarmMode::View => {}
        }
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

    fn draw_view(&self, ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.set_char(0, b'A');
        hw.set_char(1, b'L');
        // While the alarm rings the face shows the current time in full (with
        // seconds): a coherent live clock to glance at when the watch goes
        // off. Otherwise it shows the static alarm time, without seconds.
        let ringing = self.ring_until.is_some();
        let t = ctx.time;
        let (hour, minute, second, pm) = if ringing {
            (t.hour, t.minute, Some(t.second), t.hour >= 12)
        } else {
            (self.hour, self.minute, None, self.hour >= 12)
        };
        let hour = if ctx.h24 { hour } else { to_12h(hour) };
        if hour >= 10 {
            hw.set_digit(4, hour / 10);
        } else {
            hw.clear_digit(4);
        }
        hw.set_digit(5, hour % 10);
        hw.set_digit(6, minute / 10);
        hw.set_digit(7, minute % 10);
        if let Some(second) = second {
            hw.set_digit(8, second / 10);
            hw.set_digit(9, second % 10);
        } else {
            hw.clear_digit(8);
            hw.clear_digit(9);
        }
        // The Bell indicator blinks at 2 Hz while the alarm rings, to draw
        // the eye; otherwise it just reflects the enabled state.
        let bell = if ringing { (t.ms / 500) % 2 == 0 } else { self.enabled };
        hw.set_indicator(Indicator::Bell, bell);
        hw.set_indicator(Indicator::H24, ctx.h24);
        hw.set_indicator(Indicator::Pm, !ctx.h24 && pm);
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
            SimpleAlarmMode::Edit(f) => f,
            SimpleAlarmMode::View => return,
        };

        match field {
            SimpleAlarmField::Hour => {
                hw.set_char(0, b'H');
                hw.set_char(1, b'O');
            }
            SimpleAlarmField::Minute => {
                hw.set_char(0, b'M');
                hw.set_char(1, b'I');
            }
        }

        self.draw_pair(hw, 4, self.hour, field != SimpleAlarmField::Hour || blink);
        self.draw_pair(hw, 6, self.minute, field != SimpleAlarmField::Minute || blink);
        hw.clear_digit(8);
        hw.clear_digit(9);

        hw.set_indicator(Indicator::Bell, self.enabled);
        hw.set_indicator(Indicator::H24, false);
        hw.set_indicator(Indicator::Pm, false);
        hw.set_indicator(Indicator::Signal, false);
    }
}

impl Face for SimpleAlarm {
    fn init(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) {
        // Entering the face always starts in the view mode. The ring is also
        // silenced: an auto-switch (when the alarm fires) does NOT call init,
        // so the ring survives that path.
        self.mode = SimpleAlarmMode::View;
        self.ring_until = None;
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        // Ring while inside the window; go silent once we pass it. Only the
        // active face ticks, so a ring dies the moment the user switches faces
        // with Mode.
        let t = ctx.time;
        let secs = t.hour as u32 * 3600 + t.minute as u32 * 60 + t.second as u32;
        match self.ring_until {
            Some(until) if secs < until => hw.beep(),
            Some(_) => self.ring_until = None,
            None => {}
        }
        match self.mode {
            SimpleAlarmMode::View => self.draw_view(ctx, hw),
            SimpleAlarmMode::Edit(_) => self.draw_edit(&t, hw),
        }
    }

    fn background_tick(&mut self, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        let t = ctx.time;
        let fired = (t.hour, t.minute);
        // `t.second == 0` pins the ring to the top of the minute: the alarm
        // starts exactly at HH:MM:00, never in the middle of a minute (a seed
        // "from the current time" drops the seconds, so a freshly set alarm
        // does not ring mid-minute).
        if self.enabled && self.hour == t.hour && self.minute == t.minute && t.second == 0 {
            if self.last_fired != Some(fired) {
                self.last_fired = Some(fired);
                // Snap to the view mode: an alarm firing interrupts whatever
                // the face was doing (even its own edit session).
                self.mode = SimpleAlarmMode::View;
                let secs = t.hour as u32 * 3600 + t.minute as u32 * 60 + t.second as u32;
                self.ring_until = Some(secs + RING_SECS);
                return true;
            }
        } else {
            self.last_fired = None;
        }
        false
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        // Any button press silences the ring (the stock Casio behaviour).
        self.ring_until = None;
        match event.button {
            ButtonId::Alarm => {
                match self.mode {
                    SimpleAlarmMode::View => {
                        // The stock Casio: the Alarm button toggles the alarm.
                        if event.kind == GestureKind::Press {
                            self.enabled = !self.enabled;
                        }
                    }
                    SimpleAlarmMode::Edit(_) => {
                        let at = ctx.time.secs * 1000 + ctx.time.ms as u64;
                        match event.kind {
                            // A plain press adds one unit.
                            GestureKind::Press => {
                                self.step(1);
                                self.changed_at = Some(at);
                            }
                            // Holding the button resets the value to zero.
                            GestureKind::Hold => {
                                self.reset();
                                self.changed_at = Some(at);
                            }
                            // A double press adds five units in total: the
                            // first press of the pair already added one, so
                            // the second adds four more.
                            GestureKind::Double => {
                                self.step(4);
                                self.changed_at = Some(at);
                            }
                        }
                    }
                }
                // Always consumed: the Alarm button never toggles the format
                // or the chime on this face.
                true
            }
            ButtonId::Light => {
                match self.mode {
                    // A press enters the settings (no hold needed).
                    SimpleAlarmMode::View => {
                        if event.kind == GestureKind::Press {
                            self.mode = SimpleAlarmMode::Edit(SimpleAlarmField::Hour);
                            self.prime_from_now(&ctx.time);
                        }
                    }
                    SimpleAlarmMode::Edit(field) => {
                        match event.kind {
                            // A press moves to the next field; on the last one
                            // (minute) it exits the settings.
                            GestureKind::Press => {
                                self.mode = match field {
                                    SimpleAlarmField::Hour => {
                                        SimpleAlarmMode::Edit(SimpleAlarmField::Minute)
                                    }
                                    SimpleAlarmField::Minute => SimpleAlarmMode::View,
                                };
                            }
                            // A hold exits the settings.
                            GestureKind::Hold => self.mode = SimpleAlarmMode::View,
                            _ => {}
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn chord(&mut self, event: ChordEvent, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        // Only the Alarm + Light combination does anything here (in either
        // press order): it re-seeds the alarm with the current time.
        let both = [event.first, event.second];
        if !both.contains(&ButtonId::Alarm) || !both.contains(&ButtonId::Light) {
            return false;
        }
        self.seed_now(&ctx.time);
        self.changed_at = Some(ctx.time.secs * 1000 + ctx.time.ms as u64);
        // In the view mode the chord also enters the settings, so the seeded
        // time can be nudged with plain Alarm presses right away.
        if self.mode == SimpleAlarmMode::View {
            self.mode = SimpleAlarmMode::Edit(SimpleAlarmField::Hour);
        }
        true
    }
}
