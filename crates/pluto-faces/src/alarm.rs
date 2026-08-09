//! Alarm face: one alarm per weekday (Sun..Sat).
//!
//! View mode shows `AL` in the weekday letters and the number of enabled
//! alarms in the day-of-month digits (pos 2-3). In the view mode the Alarm
//! button toggles the hourly chime (the "signal", shown by the SIG
//! indicator); the Bell indicator just reflects whether any alarm is enabled.
//! Pressing Light enters the edit mode (like
//! the stock Casio: a press of the adjust key, no long-press needed): the
//! focused value blinks, Light moves to the next field
//! (day -> hour -> minute -> status), Alarm changes the value. Pressing
//! Light on the status field exits back to the view mode (a long press also
//! exits at any point). Entering the edit mode seeds an alarm that was never
//! configured (disabled at 00:00) with the current wall-clock time as the
//! base value; a configured alarm keeps its saved time. Pressing Alarm and
//! Light together re-seeds the selected day with the current time. The status
//! (AC) is shown as `ON` / `OF` in the seconds digits. While an alarm rings the
//! face shows the current time in full (with seconds) and blinks the Bell
//! indicator at 2 Hz.

use pluto_core::face::{ButtonId, ChordEvent, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::font::{Indicator, FONT};
use pluto_core::hardware::RING_BEEP;
use pluto_core::{DigitDisplay, Hardware};

/// Two-letter weekday abbreviations, indexed by `DateTime::weekday`
/// (0 = Sunday).
const WEEKDAYS: [[u8; 2]; 7] = [*b"SU", *b"MO", *b"TU", *b"WE", *b"TH", *b"FR", *b"SA"];

/// Convert a 0..23 hour into 12-hour form (0 -> 12, 13..=23 -> 1..=11), for
/// the live clock shown in the view mode.
const fn to_12h(hour: u8) -> u8 {
    match hour {
        0 => 12,
        13..=23 => hour - 12,
        _ => hour,
    }
}

/// 7-segment masks for the letters drawn in the seconds digits (ON / OF).
const SEG_O: u8 = 0x3f; // A..F, same as 0
// Lowercase "n" (C E G): reads as "On". A capital N cannot be drawn on a
// 7-segment display; "OH" (B C E F G) looked like an H.
const SEG_N: u8 = 0x54;
const SEG_F: u8 = 0x71; // A E F G

/// How long a fired alarm keeps ringing (seconds), unless silenced by a
/// button press first.
const RING_SECS: u32 = 120;

/// After a change of the edited value the focus is shown steadily (no
/// blinking) for this long, so fast scrolling via the hold auto-repeat or a
/// double press stays readable.
const NO_BLINK_MS: u64 = 750;

/// A single alarm: a time of day plus an on/off switch.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AlarmDay {
    pub enabled: bool,
    pub hour: u8,
    pub minute: u8,
}
impl Default for AlarmDay {
    fn default() -> Self {
        AlarmDay {
            enabled: false,
            hour: 0,
            minute: 0,
        }
    }
}

/// Which value is being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AlarmField {
    Day,
    Hour,
    Minute,
    Status,
}

/// Display mode of the face.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AlarmMode {
    View,
    Edit(AlarmField),
}

/// The alarm face.
#[derive(Clone, Copy, PartialEq)]
pub struct Alarm {
    /// One alarm per day, indexed by weekday (0 = Sunday).
    alarms: [AlarmDay; 7],
    mode: AlarmMode,
    /// Day currently shown / edited (0 = Sunday).
    day: usize,
    /// Last alarm that fired `(weekday, hour, minute)`, to fire only once.
    last_fired: Option<(u8, u8, u8)>,
    /// When the currently ringing alarm should go silent: `(weekday,
    /// seconds-of-day)` at which the ring ends. `None` when not ringing.
    ring_until: Option<(u8, u32)>,
    /// Seconds-of-day of the last ring beep, so the ring re-triggers the
    /// [`RING_BEEP`] melody once per second (the stock F-91W cadence) instead
    /// of on every 250 ms tick.
    last_beep: Option<u32>,
    /// Epoch ms of the last change of the edited value, to suppress blinking
    /// while the user is rapidly scrolling. `None` when unchanged.
    changed_at: Option<u64>,
}

impl Alarm {
    pub const fn new() -> Self {
        Alarm {
            alarms: [AlarmDay {
                enabled: false,
                hour: 0,
                minute: 0,
            }; 7],
            mode: AlarmMode::View,
            day: 0,
            last_fired: None,
            ring_until: None,
            last_beep: None,
            changed_at: None,
        }
    }

    pub fn alarms(&self) -> &[AlarmDay; 7] {
        &self.alarms
    }

    fn active_count(&self) -> usize {
        self.alarms.iter().filter(|a| a.enabled).count()
    }

    /// Seed the selected day's alarm with the current wall-clock time as the
    /// base value for editing, but only if the alarm was never configured:
    /// an existing alarm (enabled or set to a time) keeps its saved time, so
    /// re-entering the edit mode does not clobber it. The user can force a
    /// re-seed with the current time via the Alarm + Light chord. Seconds are
    /// dropped: the alarm fires at the top of the minute, never in the middle
    /// of one.
    fn prime_from_now(&mut self, t: &pluto_core::time::DateTime) {
        let a = &mut self.alarms[self.day];
        if !a.enabled && a.hour == 0 && a.minute == 0 {
            a.hour = t.hour;
            a.minute = t.minute;
        }
    }

    /// Unconditionally set the selected day's alarm to the current wall-clock
    /// time (the Alarm + Light chord). Seconds are dropped, see
    /// [`Self::prime_from_now`].
    fn seed_now(&mut self, t: &pluto_core::time::DateTime) {
        let a = &mut self.alarms[self.day];
        a.hour = t.hour;
        a.minute = t.minute;
    }

    fn next_field(f: AlarmField) -> Option<AlarmField> {
        match f {
            AlarmField::Day => Some(AlarmField::Hour),
            AlarmField::Hour => Some(AlarmField::Minute),
            AlarmField::Minute => Some(AlarmField::Status),
            // The status ("switcher") is the last field: the next Light
            // press exits the edit mode instead of wrapping around.
            AlarmField::Status => None,
        }
    }

    fn step(&mut self, steps: u8) {
        let a = &mut self.alarms[self.day];
        match self.mode {
            AlarmMode::Edit(AlarmField::Day) => self.day = (self.day + steps as usize) % 7,
            AlarmMode::Edit(AlarmField::Hour) => a.hour = (a.hour + steps) % 24,
            AlarmMode::Edit(AlarmField::Minute) => a.minute = (a.minute + steps) % 60,
            AlarmMode::Edit(AlarmField::Status) => a.enabled = !a.enabled,
            AlarmMode::View => {}
        }
    }

    /// Reset the focused value to zero (holding the Alarm button). The
    /// status field is binary, so "zero" means off.
    fn reset(&mut self) {
        let a = &mut self.alarms[self.day];
        match self.mode {
            AlarmMode::Edit(AlarmField::Day) => self.day = 0,
            AlarmMode::Edit(AlarmField::Hour) => a.hour = 0,
            AlarmMode::Edit(AlarmField::Minute) => a.minute = 0,
            AlarmMode::Edit(AlarmField::Status) => a.enabled = false,
            AlarmMode::View => {}
        }
    }

    /// Draw the ON / OF status into the seconds digits using 7-segment masks.
    fn draw_status(&self, hw: &mut impl Hardware, visible: bool) {
        let a = &self.alarms[self.day];
        let (hi, lo) = if a.enabled { (SEG_O, SEG_N) } else { (SEG_O, SEG_F) };
        self.seg_pos(hw, 8, if visible { hi } else { 0 });
        self.seg_pos(hw, 9, if visible { lo } else { 0 });
    }

    fn seg_pos(&self, hw: &mut impl Hardware, position: usize, mask: u8) {
        for i in 0..7 {
            let com = FONT[position][i][0];
            let seg = FONT[position][i][1];
            if com < 0 || seg < 0 {
                continue;
            }
            hw.set_segment(com as u8, seg as u8, mask & (1 << i) != 0);
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
        // Number of enabled alarms in the day-of-month digits. The tens
        // digit is always 0 (max 7 alarms) and the F-91W glass cannot render
        // a proper `0` there (segments A/D/G share one wire), so it is left
        // blank like a leading-zero day.
        let n = self.active_count();
        hw.set_digit(3, (n % 10) as u8);
        hw.clear_digit(2);
        // Live clock in the time digits (like SimpleClock), so the face is
        // readable right after an alarm auto-switched it on. While the alarm
        // rings the seconds are shown in full: the face is then a coherent
        // "what time is it now" display.
        let t = ctx.time;
        let ringing = self.ring_until.is_some();
        let hour = if ctx.h24 { t.hour } else { to_12h(t.hour) };
        if hour >= 10 {
            hw.set_digit(4, hour / 10);
        } else {
            hw.clear_digit(4);
        }
        hw.set_digit(5, hour % 10);
        hw.set_digit(6, t.minute / 10);
        hw.set_digit(7, t.minute % 10);
        if ringing {
            hw.set_digit(8, t.second / 10);
            hw.set_digit(9, t.second % 10);
        } else {
            // Seconds are hidden: an alarm face shows a time, not a ticking
            // clock.
            hw.clear_digit(8);
            hw.clear_digit(9);
        }
        // The Bell indicator blinks at 2 Hz while the alarm rings, to draw
        // the eye; otherwise it just reflects the enabled state.
        let bell = if ringing { (t.ms / 500) % 2 == 0 } else { n > 0 };
        hw.set_indicator(Indicator::Bell, bell);
        hw.set_indicator(Indicator::H24, ctx.h24);
        hw.set_indicator(Indicator::Pm, !ctx.h24 && t.hour >= 12);
        hw.set_indicator(Indicator::Signal, ctx.chime);
    }

    fn draw_edit(&self, t: &pluto_core::time::DateTime, hw: &mut impl Hardware) {
        // While the user is rapidly stepping through values (hold
        // auto-repeat or a double press) the focused value is shown
        // steadily instead of blinking; blinking resumes shortly after the
        // last change.
        let now_ms = t.secs as u64 * 1000 + t.ms as u64;
        let recently_changed = self
            .changed_at
            .is_some_and(|c| now_ms.saturating_sub(c) < NO_BLINK_MS);
        // Otherwise blink at ~2 Hz: visible for 250 ms, hidden for 250 ms.
        // (The tick interval is 250 ms on both the emulator and the
        // hardware.) While the user is rapidly scrolling, the field is
        // shown steadily.
        let blink = recently_changed || (t.ms / 250) % 2 == 0;
        let field = match self.mode {
            AlarmMode::Edit(f) => f,
            AlarmMode::View => return,
        };

        // Weekday letters: the focused day (blinking), or the field label.
        match field {
            AlarmField::Day => {
                if blink {
                    hw.set_char(0, WEEKDAYS[self.day][0]);
                    hw.set_char(1, WEEKDAYS[self.day][1]);
                } else {
                    hw.clear_char(0);
                    hw.clear_char(1);
                }
            }
            AlarmField::Hour => {
                hw.set_char(0, b'H');
                hw.set_char(1, b'O');
            }
            AlarmField::Minute => {
                hw.set_char(0, b'M');
                hw.set_char(1, b'I');
            }
            AlarmField::Status => {
                hw.set_char(0, b'A');
                hw.set_char(1, b'C');
            }
        }

        let a = &self.alarms[self.day];
        self.draw_pair(hw, 4, a.hour, field != AlarmField::Hour || blink);
        self.draw_pair(hw, 6, a.minute, field != AlarmField::Minute || blink);
        self.draw_status(hw, field != AlarmField::Status || blink);

        hw.set_indicator(Indicator::Bell, a.enabled);
        hw.set_indicator(Indicator::H24, false);
        hw.set_indicator(Indicator::Pm, false);
        hw.set_indicator(Indicator::Signal, false);
    }
}

impl Face for Alarm {
    fn init(&mut self, _ctx: &FaceContext, hw: &mut impl Hardware) {
        // Entering the face always starts in the view mode: a Mode exit from
        // the middle of an edit session must not resume the edit later. The
        // ring is also silenced: an auto-switch (when the alarm fires) does
        // NOT call init, so the ring survives that path.
        self.mode = AlarmMode::View;
        self.ring_until = None;
        self.last_beep = None;
        hw.stop_melody();
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        // Ring while inside the window; go silent once we pass it. Only the
        // active face ticks, so a ring dies the moment the user switches
        // faces with Mode. The ring melody re-triggers once per second (not
        // on every tick).
        let t = ctx.time;
        let secs = t.hour as u32 * 3600 + t.minute as u32 * 60 + t.second as u32;
        match self.ring_until {
            Some((wd, until)) if t.weekday as u8 == wd && secs < until => {
                if self.last_beep != Some(secs) {
                    self.last_beep = Some(secs);
                    hw.melody(&RING_BEEP);
                }
            }
            Some(_) => {
                self.ring_until = None;
                self.last_beep = None;
                hw.stop_melody();
            }
            None => {}
        }
        match self.mode {
            AlarmMode::View => self.draw_view(ctx, hw),
            AlarmMode::Edit(_) => self.draw_edit(&t, hw),
        }
    }

    fn background_tick(&mut self, ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        let t = ctx.time;
        let day = t.weekday as u8;
        let a = &self.alarms[day as usize];
        let fired = (day, t.hour, t.minute);
        // `t.second == 0` pins the ring to the top of the minute: the alarm
        // starts exactly at HH:MM:00, never in the middle of a minute (a seed
        // "from the current time" drops the seconds, so a freshly set alarm
        // does not ring mid-minute).
        if a.enabled && a.hour == t.hour && a.minute == t.minute && t.second == 0 {
            if self.last_fired != Some(fired) {
                self.last_fired = Some(fired);
                // Snap to view mode: an alarm firing interrupts whatever the
                // face was doing (even its own edit session).
                self.mode = AlarmMode::View;
                let secs = t.hour as u32 * 3600 + t.minute as u32 * 60 + t.second as u32;
                self.ring_until = Some((day, secs + RING_SECS));
                return true;
            }
        } else {
            self.last_fired = None;
        }
        false
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        // Any button press silences the ring (the stock Casio behaviour).
        self.ring_until = None;
        self.last_beep = None;
        hw.stop_melody();
        match event.button {
            ButtonId::Alarm => {
                // In the edit mode everything is consumed here: the Alarm
                // button never toggles the 12/24h format (nor the chime).
                if let AlarmMode::Edit(_) = self.mode {
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
                        // A double press adds five units in total: the first
                        // press of the pair already added one, so the second
                        // adds four more. The status field is binary, so the
                        // second press is ignored there (the first already
                        // toggled it).
                        GestureKind::Double
                            if !matches!(self.mode, AlarmMode::Edit(AlarmField::Status)) =>
                        {
                            self.step(4);
                            self.changed_at = Some(at);
                        }
                        _ => {}
                    }
                    // When scrolling through days, prime a never-configured
                    // day's alarm with the current time.
                    if matches!(self.mode, AlarmMode::Edit(AlarmField::Day)) {
                        self.prime_from_now(&ctx.time);
                    }
                    true
                } else {
                    // In the view mode the press is not consumed, so the
                    // runtime toggles the hourly chime instead (see
                    // `alarm_action`).
                    false
                }
            }
            ButtonId::Light => {
                match self.mode {
                    // A press enters the edit mode (no hold needed).
                    AlarmMode::View => {
                        if event.kind == GestureKind::Press {
                            self.mode = AlarmMode::Edit(AlarmField::Day);
                            self.prime_from_now(&ctx.time);
                        }
                    }
                    AlarmMode::Edit(field) => {
                        match event.kind {
                            // A press moves to the next field; on the last
                            // one (status) it exits the edit mode.
                            GestureKind::Press => {
                                self.mode = match Self::next_field(field) {
                                    Some(f) => AlarmMode::Edit(f),
                                    None => AlarmMode::View,
                                };
                            }
                            // A hold exits the edit mode.
                            GestureKind::Hold => self.mode = AlarmMode::View,
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
        // press order): it re-seeds the selected day with the current time.
        let both = [event.first, event.second];
        if !both.contains(&ButtonId::Alarm) || !both.contains(&ButtonId::Light) {
            return false;
        }
        if let AlarmMode::Edit(_) = self.mode {
            self.seed_now(&ctx.time);
            self.changed_at = Some(ctx.time.secs * 1000 + ctx.time.ms as u64);
            true
        } else {
            false
        }
    }

    fn alarm_action(&self) -> pluto_core::face::AlarmAction {
        pluto_core::face::AlarmAction::ChimeToggle
    }
}
