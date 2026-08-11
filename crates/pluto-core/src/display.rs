//! LCD glass abstraction.
//!
//! Coordinates passed to [`Display`] are *glass* coordinates: the same
//! `(com, seg)` values used by the emulator skin and by the font tables in
//! [`crate::font`]. Each concrete driver translates them to the real
//! LCD RAM bits through the configurable `display_map`.

use crate::font::{Indicator, INDICATORS, FONT};
use crate::letters::{LETTER_POS_SEGS, LETTER_SEGS};

/// A driver for the F-91W LCD glass.
pub trait Display {
    /// Turn every segment off.
    fn clear_all(&mut self);
    /// Turn a single glass segment on (`on == true`) or off.
    fn set_segment(&mut self, com: u8, seg: u8, on: bool);
}

/// Convenience drawing helpers available for any [`Display`].
pub trait DigitDisplay: Display {
    /// Draw a digit (0-9) at one of the 10 7-segment positions.
    ///
    /// Position layout: 0-1=weekday letters, 2-3=day, 4-5=hours,
    /// 6-7=minutes, 8-9=seconds.
    fn set_digit(&mut self, position: u8, digit: u8);
    /// Draw a character at a weekday (mode) digit, positions 0 or 1. The stock
    /// F-91W uses these to show the day of the week (SU, MO, TU, ...) or the
    /// mode labels AL / ST. Accepts any ASCII `' '` .. `'~'`; which segments
    /// light is defined per character in `letters/letters.json` (see
    /// [`crate::letters`]). Characters without a definition render blank.
    fn set_char(&mut self, position: u8, ch: u8);
    /// Turn off every segment of a digit position.
    fn clear_digit(&mut self, position: u8);
    /// Clear a weekday (mode) digit, including its extra segments.
    fn clear_char(&mut self, position: u8);
    /// Turn an indicator segment on or off.
    fn set_indicator(&mut self, indicator: Indicator, on: bool);
}

impl<T: Display + ?Sized> DigitDisplay for T {
    fn set_digit(&mut self, position: u8, digit: u8) {
        let pos = position as usize;
        if pos >= FONT.len() || digit > 9 {
            return;
        }
        let segs = crate::font::DIGIT_SEGS[digit as usize];
        for pass in 0..2 {
            for i in 0..7 {
                let on = segs & (1 << i) != 0;
                if on != (pass == 1) {
                    continue;
                }
                let com = FONT[pos][i][0];
                let seg = FONT[pos][i][1];
                if com < 0 || seg < 0 {
                    continue;
                }
                self.set_segment(com as u8, seg as u8, on);
            }
        }
    }

    /// Turn off every segment of a digit position.
    fn clear_digit(&mut self, position: u8) {
        let pos = position as usize;
        if pos >= FONT.len() {
            return;
        }
        for i in 0..7 {
            let com = FONT[pos][i][0];
            let seg = FONT[pos][i][1];
            if com < 0 || seg < 0 {
                continue;
            }
            self.set_segment(com as u8, seg as u8, false);
        }
    }

    fn set_char(&mut self, position: u8, ch: u8) {
        let pos = position as usize;
        if pos > 1 || ch < b' ' || ch > b'~' {
            return;
        }
        // Clear every segment of this weekday digit, then draw the ones the
        // character definition says to light.
        for s in LETTER_POS_SEGS[pos] {
            self.set_segment(s.0 as u8, s.1 as u8, false);
        }
        for s in LETTER_SEGS[(ch - b' ') as usize][pos] {
            self.set_segment(s.0 as u8, s.1 as u8, true);
        }
    }

    /// Clear a weekday (mode) digit, including its extra segments.
    fn clear_char(&mut self, position: u8) {
        let pos = position as usize;
        if pos > 1 {
            return;
        }
        for s in LETTER_POS_SEGS[pos] {
            self.set_segment(s.0 as u8, s.1 as u8, false);
        }
    }

    fn set_indicator(&mut self, indicator: Indicator, on: bool) {
        let i = indicator as usize;
        self.set_segment(INDICATORS[i][0], INDICATORS[i][1], on);
    }
}
