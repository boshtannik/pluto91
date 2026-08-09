//! Simple clock face: weekday + date on top, HH:MM:SS in the middle. Beeps on
//! the Alarm button (the 12/24-hour format toggle).

use pluto_core::face::{ButtonId, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::font::Indicator;
use pluto_core::{DigitDisplay, Hardware};

/// Two-letter weekday abbreviations, indexed by `DateTime::weekday`
/// (0 = Sunday). Rendered in the weekday (mode) digits of the F-91W.
const WEEKDAYS: [[u8; 2]; 7] = [*b"SU", *b"MO", *b"TU", *b"WE", *b"TH", *b"FR", *b"SA"];

/// Convert a 0..23 hour into 12-hour form (0 -> 12, 13..=23 -> 1..=11).
const fn to_12h(hour: u8) -> u8 {
    match hour {
        0 => 12,
        13..=23 => hour - 12,
        _ => hour,
    }
}

/// Shows the time and the date.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct SimpleClock {}

impl SimpleClock {
    pub const fn new() -> Self {
        SimpleClock {}
    }
}

impl Face for SimpleClock {
    fn init(&mut self, _ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.set_segment(1, 16, true);
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        let t = ctx.time;

        // weekday letters (positions 0-1)
        let wd = WEEKDAYS[t.weekday as usize % 7];
        hw.set_char(0, wd[0]);
        hw.set_char(1, wd[1]);

        // day of month in the top two digits, no leading zero
        if t.day >= 10 {
            hw.set_digit(2, t.day / 10);
        } else {
            hw.clear_digit(2);
        }
        hw.set_digit(3, t.day % 10);

        // HH:MM:SS, no leading zero on the hour; 12h format shows 12:xx for
        // midnight and 1..11 otherwise.
        let hour = if ctx.h24 { t.hour } else { to_12h(t.hour) };
        if hour >= 10 {
            hw.set_digit(4, hour / 10);
        } else {
            hw.clear_digit(4);
        }
        hw.set_digit(5, hour % 10);
        hw.set_digit(6, t.minute / 10);
        hw.set_digit(7, t.minute % 10);
        hw.set_digit(8, t.second / 10);
        hw.set_digit(9, t.second % 10);

        // blinking colon
        // hw.set_segment(1, 16, t.second % 2 == 0);

        hw.set_indicator(Indicator::H24, ctx.h24);
        hw.set_indicator(Indicator::Pm, !ctx.h24 && t.hour >= 12);
        hw.set_indicator(Indicator::Signal, ctx.chime);
    }

    fn button(&mut self, event: GestureEvent, _ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        if event.button == ButtonId::Alarm && event.kind == GestureKind::Press {
            hw.beep();
        }
        false
    }
}
