//! High-level LCD helpers, mirroring the pluto firmware's `svc/lcd.c`:
//! printing integers and text onto the F-91W glass.
//!
//! Positions 0-7 are the seven-segment digits; positions 8-9 are the two
//! weekday/mode letters. Text on the digit positions uses the pluto "normal"
//! 7-segment map (letters `a-z` drawn as their 7-segment approximations).

use crate::display::{DigitDisplay, Display, Indicator};

/// Character -> segment mask for the digit positions, straight from pluto's
/// `common/svc/maps/normal.map`. Index: `'0'-'9'` = 0-9, `'a'-'z'` = 10-35,
/// `'-'` = 36, `'/'` = 37. Bit 0 = A ... bit 6 = G.
pub const NORMAL_MAP: [u8; 38] = [
    0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f, // 0-9
    0x77, 0x7c, 0x58, 0x5e, 0x79, 0x71, 0x3d, 0x74, 0x10, 0x1e, // a-j
    0x72, 0x38, 0x37, 0x54, 0x5c, 0x73, 0x67, 0x50, 0x6d, 0x78, // k-t
    0x1c, 0x3c, 0x3e, 0x56, 0x6e, 0x5b,                         // u-z
    0x40, 0x52, // '-' '/'
];

/// Map a character to its index in [`NORMAL_MAP`] (0 for anything unknown).
const fn normal_index(c: u8) -> usize {
    match c {
        b'0'..=b'9' => (c - b'0') as usize,
        b'a'..=b'z' => 10 + (c - b'a') as usize,
        b'A'..=b'Z' => 10 + (c - b'A') as usize,
        b'-' => 36,
        b'/' => 37,
        _ => 0,
    }
}

/// A borrowed view of a [`Display`] with the pluto drawing helpers.
pub struct Lcd<'a, D: Display> {
    d: &'a mut D,
}

impl<'a, D: Display> Lcd<'a, D> {
    pub fn new(d: &'a mut D) -> Self {
        Lcd { d }
    }

    /// Turn every segment off.
    pub fn clear(&mut self) {
        self.d.clear_all();
    }

    /// Draw an integer `value` as `len` digits starting at `dig`, right
    /// aligned and zero-padded (pluto's `svc_lcd_puti`).
    pub fn puti(&mut self, dig: u8, len: u8, value: u32) {
        let mut value = value;
        let mut dig = dig.saturating_add(len.saturating_sub(1));
        let mut len = len;
        while len > 0 {
            self.d.set_digit(dig, (value % 10) as u8);
            value /= 10;
            len -= 1;
            dig = dig.wrapping_sub(1);
        }
    }

    /// Like [`Lcd::puti`] but hexadecimal (pluto's `svc_lcd_putix`).
    pub fn putix(&mut self, dig: u8, len: u8, value: u32) {
        let mut value = value;
        let mut dig = dig.saturating_add(len.saturating_sub(1));
        let mut len = len;
        while len > 0 {
            self.d.set_digit(dig, (value % 16) as u8);
            value /= 16;
            len -= 1;
            dig = dig.wrapping_sub(1);
        }
    }

    /// Draw a single character at `dig`: digits/letters on the 7-segment
    /// positions (0-7) via [`NORMAL_MAP`], letters on the two letter
    /// positions (8-9). A space turns every segment of the position off
    /// (pluto's `svc_lcd_putc`, used to blank unused digits).
    pub fn putc(&mut self, dig: u8, c: u8) {
        if c == b' ' {
            if dig >= 8 {
                self.d.clear_char(dig);
            } else {
                self.d.clear_digit(dig);
            }
            return;
        }
        if dig >= 8 {
            self.d.set_char(dig, c);
            return;
        }
        let segs = NORMAL_MAP[normal_index(c)];
        for i in 0..7 {
            let com = crate::display::FONT[dig as usize][i][0];
            let seg = crate::display::FONT[dig as usize][i][1];
            if com < 0 || seg < 0 {
                continue;
            }
            self.d.set_segment(com as u8, seg as u8, segs & (1 << i) != 0);
        }
    }

    /// Draw a string starting at `dig`.
    pub fn puts(&mut self, dig: u8, s: &[u8]) {
        for (i, c) in s.iter().enumerate() {
            self.putc(dig + i as u8, *c);
        }
    }

    /// Turn an indicator on or off.
    pub fn indicator(&mut self, ind: Indicator, on: bool) {
        self.d.set_indicator(ind, on);
    }
}
