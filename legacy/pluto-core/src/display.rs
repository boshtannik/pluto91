//! F-91W glass abstraction, in pluto coordinates.
//!
//! The pluto firmware uses a position-based LCD model: positions 0-7 are the
//! eight 7-segment digits (hours, minutes, seconds, day-of-month, left to
//! right), positions 8-9 are the two weekday/mode letters at the top. Each
//! position maps to the same physical glass the opencasio firmware drives, so
//! the `(com, seg)` glass coordinates below are shared with the emulator skin.
//!
//! The digit positions here are derived from the opencasio display map (same
//! F-91W glass): hour_1/hour_2 = positions 0/1, minute_1/minute_2 = 2/3,
//! second_1/second_2 = 4/5, day_1/day_2 = 6/7.

/// A driver for the F-91W LCD glass. Coordinates are `(com, seg)`.
pub trait Display {
    /// Turn every segment off.
    fn clear_all(&mut self);
    /// Turn a single glass segment on (`on == true`) or off.
    fn set_segment(&mut self, com: u8, seg: u8, on: bool);
}

/// The seven segments `A..G` of each digit position as `(com, seg)` glass
/// coordinates. `(-1, -1)` means the segment is shared with a neighbouring
/// digit (already driven by the other digit).
///
/// Positions: 0=hour tens, 1=hour ones, 2=minute tens, 3=minute ones,
/// 4=second tens, 5=second ones, 6=day tens, 7=day ones.
pub const FONT: [[[i8; 2]; 7]; 8] = [
    /* pos 0 */ [[1, 18], [2, 19], [0, 19], [1, 18], [0, 18], [2, 18], [1, 19]],
    /* pos 1 */ [[2, 20], [2, 21], [1, 21], [0, 21], [0, 20], [1, 17], [1, 20]],
    /* pos 2 */ [[0, 22], [2, 23], [0, 23], [0, 22], [1, 22], [2, 22], [1, 23]],
    /* pos 3 */ [[2, 1], [2, 10], [0, 1], [0, 0], [1, 0], [2, 0], [1, 1]],
    /* pos 4 */ [[2, 2], [2, 3], [0, 4], [0, 3], [0, 2], [1, 2], [1, 3]],
    /* pos 5 */ [[2, 4], [2, 5], [1, 6], [0, 6], [0, 5], [1, 4], [1, 5]],
    /* pos 6 */ [[1, 9], [0, 9], [2, 9], [1, 9], [0, 10], [-1, -1], [1, 9]],
    /* pos 7 */ [[0, 7], [1, 7], [2, 7], [2, 6], [2, 8], [0, 8], [1, 8]],
];

/// Which of the seven segments `A..G` are lit for each digit, as a bitmask
/// (bit 0 = A ... bit 6 = G).
pub const DIGIT_SEGS: [u8; 10] = [
    0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f,
];

/// The indicator segments (beyond the digits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Indicator {
    /// The colon between hour and minute.
    Colon = 0,
    /// The alarm bell symbol.
    Bell = 1,
    Pm = 2,
    H24 = 3,
    Lap = 4,
    /// The "signal" bars segment.
    Bars = 5,
}

/// Glass coordinates `(com, seg)` of the indicators.
pub const INDICATORS: [[u8; 2]; 6] = [
    [1, 16], /* Colon  (dot_down + dot_up)      */
    [0, 16], /* Bell   (alarm_inside)           */
    [2, 17], /* PM     (timemode_PM)            */
    [2, 16], /* 24H    (timemode_24H)           */
    [1, 10], /* LAP    (lap)                    */
    [0, 17], /* Bars   (signal_1..signal_5)     */
];

/// Convenience drawing helpers available for any [`Display`].
pub trait DigitDisplay: Display {
    /// Draw a digit (0-9) at one of the 8 seven-segment positions.
    fn set_digit(&mut self, position: u8, digit: u8);
    /// Turn off every segment of a digit position.
    fn clear_digit(&mut self, position: u8);
    /// Draw a character at one of the two letter positions (8 or 9).
    fn set_char(&mut self, position: u8, ch: u8);
    /// Clear a letter position, including its extra segments.
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
        let segs = DIGIT_SEGS[digit as usize];
        for i in 0..7 {
            let com = FONT[pos][i][0];
            let seg = FONT[pos][i][1];
            if com < 0 || seg < 0 {
                continue;
            }
            self.set_segment(com as u8, seg as u8, segs & (1 << i) != 0);
        }
    }

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
        let pos = (position as usize).saturating_sub(8);
        if pos > 1 || ch < b' ' || ch > b'~' {
            return;
        }
        for s in crate::letters::LETTER_POS_SEGS[pos] {
            self.set_segment(s.0 as u8, s.1 as u8, false);
        }
        for s in crate::letters::LETTER_SEGS[(ch - b' ') as usize][pos] {
            self.set_segment(s.0 as u8, s.1 as u8, true);
        }
    }

    fn clear_char(&mut self, position: u8) {
        let pos = (position as usize).saturating_sub(8);
        if pos > 1 {
            return;
        }
        for s in crate::letters::LETTER_POS_SEGS[pos] {
            self.set_segment(s.0 as u8, s.1 as u8, false);
        }
    }

    fn set_indicator(&mut self, indicator: Indicator, on: bool) {
        let i = indicator as usize;
        self.set_segment(INDICATORS[i][0], INDICATORS[i][1], on);
    }
}
