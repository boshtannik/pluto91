//! Calendar face: a perpetual calendar for the years 2000..=2099.
//!
//! The view mode shows the configured date: the weekday in the top-left
//! letters, the day of the month in the top two digits, the **year** in the
//! big digits (4-7) and the month number in the seconds digits (8-9). The
//! weekday is **computed** from the date, so scrolling through dates shows you
//! which day of the week they fall on. On the very first entry the face is
//! seeded with the current date.
//!
//! Light enters the settings: the fields are year -> month -> day -> hour ->
//! minute -> second (Light advances, Alarm steps the value, a double adds 5, a
//! hold resets to the minimum — 2000 / 01 / 01 / 00 / 00 / 00), like the other
//! faces. The year always stays in 2000..=2099 (stepping past 2099 wraps to
//! 2000). Each field shows its two-letter label in the top-left letters —
//! **YR** / **MO** / **DA** for the date fields and **HR** / **MI** / **SE**
//! for the time fields — while the edited *value* blinks in place. The weekday
//! is always a computed value (shown in the view, never edited). While the time
//! fields are edited the whole clock shows in its native places (HH:MM:SS) with
//! only the focused part blinking. The day is
//! always clamped to the real length of the chosen
//! month/year: switching to February folds a 30th/31st down to the 28th/29th,
//! and changing to a non-leap year folds Feb 29 to the 28th.
//!
//! The face doubles as the watch's date/time setting screen: the hour, minute
//! and second fields (labels `HR` / `MI` / `SE`, edited in 24-hour time)
//! seed from the current clock, and **leaving the settings writes the whole
//! date + time back to the wall clock** (via [`Hardware::set_rtc`]): on the
//! emulator it re-tunes `js_now`, on the hardware it programs the RTC.
//!
//! The Alarm + Light chord enters an **info mode**: the first chord shows
//! `YR` and the number of days in the current year (365/366) in the big
//! digits; the second shows `MO` and the number of days in the current month
//! in the seconds digits; a third chord (or a timeout of a few seconds
//! without any press) returns to the view. A chord pressed during an edit
//! keeps the settings (the clamping has already made them valid) and jumps to
//! the info mode right away.

use pluto_core::face::{ButtonId, ChordEvent, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::font::Indicator;
use pluto_core::time::{days_in_month, days_in_year, epoch_ms_of, weekday_of};
use pluto_core::{DigitDisplay, Hardware};

/// Two-letter weekday abbreviations, indexed by `Weekday` (0 = Sunday).
const WEEKDAYS: [[u8; 2]; 7] = [*b"SU", *b"MO", *b"TU", *b"WE", *b"TH", *b"FR", *b"SA"];

/// Year range of the perpetual calendar.
const YEAR_MIN: u16 = 2000;
const YEAR_SPAN: u16 = 100; // 2000..=2099

/// How long the info mode stays up without a button press (seconds).
const INFO_SECS: u64 = 5;

/// After a change of the edited value the focus is shown steadily (no
/// blinking) for this long, so fast scrolling via the hold auto-repeat or a
/// double press stays readable.
const NO_BLINK_MS: u64 = 750;

/// Which value is being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CalendarField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// Display mode of the face. The two info screens are separate variants so a
/// third chord returns to the view mode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CalendarMode {
    View,
    Edit(CalendarField),
    /// Info screen: `YR` + number of days in the current year.
    InfoYr,
    /// Info screen: `MO` + number of days in the current month.
    InfoMo,
}

/// The calendar face.
#[derive(Clone, Copy, PartialEq)]
pub struct Calendar {
    /// Configured date. `year == 0` means "never configured": the face seeds
    /// itself with the current date on its first `init`.
    year: u16,
    month: u8,
    day: u8,
    /// Time-of-day fields, meaningful only while the settings are open (the
    /// view does not show them). Seeded from the wall clock each time the
    /// settings are entered, and written back to the RTC when they are left.
    hour: u8,
    minute: u8,
    second: u8,
    mode: CalendarMode,
    /// Epoch ms of the last change of the edited value, to suppress blinking
    /// while the user is rapidly scrolling. `None` when unchanged.
    changed_at: Option<u64>,
    /// Epoch second at which the info mode expires (auto-return to view).
    /// `None` when not in the info mode.
    info_until: Option<u64>,
}

impl CalendarMode {
    fn is_info(&self) -> bool {
        matches!(self, CalendarMode::InfoYr | CalendarMode::InfoMo)
    }
}

impl Calendar {
    pub const fn new() -> Self {
        Calendar {
            year: 0, // unconfigured -> seeded from the current date on init
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            mode: CalendarMode::View,
            changed_at: None,
            info_until: None,
        }
    }

    /// Seed a never-configured face with the current date, so the first entry
    /// shows "today" instead of an arbitrary value. Once set (year != 0) the
    /// configured date is kept across mode switches.
    fn seed_if_needed(&mut self, t: &pluto_core::time::DateTime) {
        if self.year == 0 {
            self.year = t.year;
            self.month = t.month as u8;
            self.day = t.day;
        }
    }

    /// Seed the time-of-day fields from the wall clock, called each time the
    /// settings are entered: the hour/minute/second fields start at the
    /// current time and only diverge as the user edits them.
    fn seed_time(&mut self, t: &pluto_core::time::DateTime) {
        self.hour = t.hour;
        self.minute = t.minute;
        self.second = t.second;
    }

    /// Write the whole configured date + time back to the wall clock, used
    /// when leaving the settings. Returns the written epoch, so callers that
    /// keep counting from the new clock (the info mode's expiry) can measure
    /// it against the time they just wrote rather than the pre-write one.
    fn apply_clock(&mut self, hw: &mut impl Hardware) -> u64 {
        let epoch = epoch_ms_of(self.year, self.month, self.day, self.hour, self.minute, self.second);
        hw.set_rtc(epoch);
        epoch
    }

    fn step(&mut self, steps: u8) {
        match self.mode {
            CalendarMode::Edit(CalendarField::Year) => {
                self.year = (self.year - YEAR_MIN + steps as u16) % YEAR_SPAN + YEAR_MIN;
                self.clamp_day();
            }
            CalendarMode::Edit(CalendarField::Month) => {
                self.month = (self.month - 1 + steps) % 12 + 1;
                self.clamp_day();
            }
            CalendarMode::Edit(CalendarField::Day) => {
                let dim = days_in_month(self.year, self.month);
                self.day = (self.day - 1 + steps) % dim + 1;
            }
            CalendarMode::Edit(CalendarField::Hour) => self.hour = (self.hour + steps) % 24,
            CalendarMode::Edit(CalendarField::Minute) => self.minute = (self.minute + steps) % 60,
            CalendarMode::Edit(CalendarField::Second) => self.second = (self.second + steps) % 60,
            CalendarMode::View | CalendarMode::InfoYr | CalendarMode::InfoMo => {}
        }
    }

    /// Reset the focused value to its minimum (holding the Alarm button):
    /// the year to 2000, the month to 01, the day to 01, the time fields to
    /// 00.
    fn reset(&mut self) {
        match self.mode {
            CalendarMode::Edit(CalendarField::Year) => self.year = YEAR_MIN,
            CalendarMode::Edit(CalendarField::Month) => self.month = 1,
            CalendarMode::Edit(CalendarField::Day) => self.day = 1,
            CalendarMode::Edit(CalendarField::Hour) => self.hour = 0,
            CalendarMode::Edit(CalendarField::Minute) => self.minute = 0,
            CalendarMode::Edit(CalendarField::Second) => self.second = 0,
            _ => {}
        }
    }

    /// Fold the day back into the real length of the chosen month/year, e.g.
    /// switching to February collapses a 31st to the 28th/29th and changing
    /// to a non-leap year collapses Feb 29 to the 28th.
    fn clamp_day(&mut self) {
        let dim = days_in_month(self.year, self.month);
        if self.day > dim {
            self.day = dim;
        }
    }

    fn draw_year(&self, hw: &mut impl Hardware) {
        hw.set_digit(4, (self.year / 1000) as u8);
        hw.set_digit(5, ((self.year / 100) % 10) as u8);
        hw.set_digit(6, ((self.year / 10) % 10) as u8);
        hw.set_digit(7, (self.year % 10) as u8);
    }

    fn draw_day(&self, hw: &mut impl Hardware) {
        if self.day >= 10 {
            hw.set_digit(2, self.day / 10);
        } else {
            hw.clear_digit(2);
        }
        hw.set_digit(3, self.day % 10);
    }

    fn draw_month(&self, hw: &mut impl Hardware) {
        hw.set_digit(8, self.month / 10);
        hw.set_digit(9, self.month % 10);
    }

    /// A 3-digit count right-aligned in the big digits (positions 5-7), for
    /// the days-in-year info (365/366).
    fn draw_count(&self, hw: &mut impl Hardware, count: u16) {
        hw.clear_digit(4);
        hw.set_digit(5, (count / 100) as u8);
        hw.set_digit(6, ((count / 10) % 10) as u8);
        hw.set_digit(7, (count % 10) as u8);
    }

    fn set_indicators(&self, hw: &mut impl Hardware, chime: bool) {
        hw.set_indicator(Indicator::Bell, false);
        hw.set_indicator(Indicator::H24, false);
        hw.set_indicator(Indicator::Pm, false);
        hw.set_indicator(Indicator::Lap, false);
        hw.set_indicator(Indicator::Signal, chime);
    }

    fn draw_view(&self, ctx: &FaceContext, hw: &mut impl Hardware) {
        let wd = weekday_of(self.year, self.month, self.day);
        hw.set_char(0, WEEKDAYS[wd as usize % 7][0]);
        hw.set_char(1, WEEKDAYS[wd as usize % 7][1]);
        self.draw_day(hw);
        self.draw_year(hw);
        self.draw_month(hw);
        self.set_indicators(hw, ctx.chime);
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
            CalendarMode::Edit(f) => f,
            _ => return,
        };

        // The top-left letters show the field's label — YR / MO / DA / HR /
        // MI / SE — steadily; the edited *value* is what blinks.
        let label = match field {
            CalendarField::Year => *b"YR",
            CalendarField::Month => *b"MO",
            CalendarField::Day => *b"DA",
            CalendarField::Hour => *b"HR",
            CalendarField::Minute => *b"MI",
            CalendarField::Second => *b"SE",
        };
        hw.set_char(0, label[0]);
        hw.set_char(1, label[1]);

        // Editing the time shows the whole clock in its native places — hours
        // at 4-5 (no leading zero, like the clock face), minutes at 6-7,
        // seconds at 8-9 — and only the focused field blinks. The day digits
        // are not part of a clock, so they are cleared while editing the time.
        if matches!(field, CalendarField::Hour | CalendarField::Minute | CalendarField::Second) {
            hw.clear_digit(2);
            hw.clear_digit(3);
            let show_hour = field != CalendarField::Hour || blink;
            if show_hour && self.hour >= 10 {
                hw.set_digit(4, self.hour / 10);
            } else {
                hw.clear_digit(4);
            }
            if show_hour {
                hw.set_digit(5, self.hour % 10);
            } else {
                hw.clear_digit(5);
            }

            let show_minute = field != CalendarField::Minute || blink;
            if show_minute {
                hw.set_digit(6, self.minute / 10);
                hw.set_digit(7, self.minute % 10);
            } else {
                hw.clear_digit(6);
                hw.clear_digit(7);
            }

            let show_second = field != CalendarField::Second || blink;
            if show_second {
                hw.set_digit(8, self.second / 10);
                hw.set_digit(9, self.second % 10);
            } else {
                hw.clear_digit(8);
                hw.clear_digit(9);
            }
            return;
        }

        // Editing the date keeps the view layout — year in the big digits,
        // day at 2-3, month at 8-9 — with only the focused value blinking.
        if field == CalendarField::Year {
            if blink {
                self.draw_year(hw);
            } else {
                hw.clear_digit(4);
                hw.clear_digit(5);
                hw.clear_digit(6);
                hw.clear_digit(7);
            }
        } else {
            self.draw_year(hw);
        }

        if field == CalendarField::Day {
            if blink {
                self.draw_day(hw);
            } else {
                hw.clear_digit(2);
                hw.clear_digit(3);
            }
        } else {
            self.draw_day(hw);
        }

        if field == CalendarField::Month {
            if blink {
                self.draw_month(hw);
            } else {
                hw.clear_digit(8);
                hw.clear_digit(9);
            }
        } else {
            self.draw_month(hw);
        }

        self.set_indicators(hw, false);
    }

    fn draw_info(&self, ctx: &FaceContext, hw: &mut impl Hardware) {
        match self.mode {
            CalendarMode::InfoYr => {
                hw.set_char(0, b'Y');
                hw.set_char(1, b'R');
                self.draw_count(hw, days_in_year(self.year));
                self.draw_day(hw);
                self.draw_month(hw);
            }
            CalendarMode::InfoMo => {
                hw.set_char(0, b'M');
                hw.set_char(1, b'O');
                self.draw_year(hw);
                self.draw_day(hw);
                hw.set_digit(8, days_in_month(self.year, self.month) / 10);
                hw.set_digit(9, days_in_month(self.year, self.month) % 10);
            }
            _ => return,
        }
        self.set_indicators(hw, ctx.chime);
    }
}

impl Face for Calendar {
    fn init(&mut self, ctx: &FaceContext, _hw: &mut impl Hardware) {
        // Entering the face always starts in the view mode (a Mode exit from
        // the middle of an edit or the info mode must not resume later) and
        // seeds a never-configured date with the current one.
        self.seed_if_needed(&ctx.time);
        self.mode = CalendarMode::View;
        self.info_until = None;
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        // The info mode auto-returns to the view after a few seconds without
        // any button press (any press resets the countdown in `button`).
        if self.mode.is_info() {
            if let Some(until) = self.info_until {
                if ctx.time.secs >= until {
                    self.mode = CalendarMode::View;
                    self.info_until = None;
                }
            }
        }
        match self.mode {
            CalendarMode::View => self.draw_view(ctx, hw),
            CalendarMode::Edit(_) => self.draw_edit(&ctx.time, hw),
            CalendarMode::InfoYr | CalendarMode::InfoMo => self.draw_info(ctx, hw),
        }
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        if self.mode.is_info() {
            // In the info mode any press just keeps it alive; a chord is the
            // only way to move on (see `chord`).
            self.info_until = Some(ctx.time.secs + INFO_SECS);
            return true;
        }
        match self.mode {
            CalendarMode::Edit(_) => match event.button {
                ButtonId::Alarm => {
                    let at = ctx.time.secs as u64 * 1000 + ctx.time.ms as u64;
                    match event.kind {
                        // A plain press adds one unit.
                        GestureKind::Press => {
                            self.step(1);
                            self.changed_at = Some(at);
                        }
                        // Holding the button resets the value to its minimum.
                        GestureKind::Hold => {
                            self.reset();
                            self.changed_at = Some(at);
                        }
                        // A double press adds five units in total: the first
                        // press of the pair already added one, so the second
                        // adds four more.
                        GestureKind::Double => {
                            self.step(4);
                            self.changed_at = Some(at);
                        }
                    }
                    true
                }
                ButtonId::Light => {
                    match event.kind {
                        // A press moves to the next field; on the last one
                        // (second) it exits the settings and writes the new
                        // date + time back to the wall clock.
                        GestureKind::Press => {
                            self.mode = match self.mode {
                                CalendarMode::Edit(CalendarField::Year) => {
                                    CalendarMode::Edit(CalendarField::Month)
                                }
                                CalendarMode::Edit(CalendarField::Month) => {
                                    CalendarMode::Edit(CalendarField::Day)
                                }
                                CalendarMode::Edit(CalendarField::Day) => {
                                    CalendarMode::Edit(CalendarField::Hour)
                                }
                                CalendarMode::Edit(CalendarField::Hour) => {
                                    CalendarMode::Edit(CalendarField::Minute)
                                }
                                CalendarMode::Edit(CalendarField::Minute) => {
                                    CalendarMode::Edit(CalendarField::Second)
                                }
                                _ => {
                                    self.apply_clock(hw);
                                    CalendarMode::View
                                }
                            };
                        }
                        // A hold exits the settings (and writes the clock).
                        GestureKind::Hold => {
                            self.apply_clock(hw);
                            self.mode = CalendarMode::View;
                        }
                        _ => {}
                    }
                    true
                }
                _ => false,
            },
            CalendarMode::View => match event.button {
                // A press enters the settings (no hold needed). The time
                // fields start from the current wall clock.
                ButtonId::Light => {
                    if event.kind == GestureKind::Press {
                        self.seed_time(&ctx.time);
                        self.mode = CalendarMode::Edit(CalendarField::Year);
                    }
                    true
                }
                // The Alarm button is not consumed: the runtime toggles the
                // global 12/24-hour format (which the calendar does not show).
                _ => false,
            },
            CalendarMode::InfoYr | CalendarMode::InfoMo => unreachable!(),
        }
    }

    fn chord(&mut self, event: ChordEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        // Only the Alarm + Light combination does anything here (in either
        // press order): it cycles view -> info YR -> info MO -> view. A chord
        // from the middle of an edit keeps the settings (they were clamped on
        // every change) and jumps straight to the info mode, applying the new
        // date + time to the wall clock as it leaves the settings.
        let both = [event.first, event.second];
        if !both.contains(&ButtonId::Alarm) || !both.contains(&ButtonId::Light) {
            return false;
        }
        // A chord from the middle of an edit writes the clock: the wall clock
        // then jumps to the written date + time, so the info countdown must be
        // measured from that new time (the old `ctx.time` would make it expire
        // immediately).
        let written_epoch = if matches!(self.mode, CalendarMode::Edit(_)) {
            Some(self.apply_clock(hw))
        } else {
            None
        };
        self.mode = match self.mode {
            CalendarMode::View => CalendarMode::InfoYr,
            CalendarMode::Edit(_) => CalendarMode::InfoYr,
            CalendarMode::InfoYr => CalendarMode::InfoMo,
            CalendarMode::InfoMo => CalendarMode::View,
        };
        self.info_until = if self.mode.is_info() {
            Some(
                written_epoch
                    .map(|epoch| epoch / 1000)
                    .unwrap_or(ctx.time.secs)
                    + INFO_SECS,
            )
        } else {
            None
        };
        true
    }
}
