//! The F-91W 7-segment font in glass coordinates.
//!
//! For each of the 10 digit positions the table gives the seven segments
//! `A..G` as `(com, seg)` glass coordinates, taken from the emulator skin.
//! `(-1, -1)` means the segment does not exist for that position (the F-91W
//! shares some segments between neighbouring digits).
//!
//! Positions: 0-1 = weekday letters (SU, MO, TU, ...), 2-3 = day of month,
//! 4-5 = hours, 6-7 = minutes, 8-9 = seconds.
//!
//! The weekday letters (positions 0-1) are *not* driven from this table: which
//! segments light per character is defined in `letters/letters.json` (see
//! [`crate::letters`]).
pub const FONT: [[[i8; 2]; 7]; 10] = [
    /* pos 0 */ [[0, 13], [1, 13], [2, 13], [2, 15], [2, 14], [0, 14], [1, 15]],
    /* pos 1 */ [[0, 11], [1, 11], [1, 11], [2, 11], [1, 12], [1, 12], [2, 12]],
    /* pos 2 */ [[1, 9], [0, 9], [2, 9], [1, 9], [0, 10], [-1, -1], [1, 9]],
    /* pos 3 */ [[0, 7], [1, 7], [2, 7], [2, 6], [2, 8], [0, 8], [1, 8]],
    /* pos 4 */ [[1, 18], [2, 19], [0, 19], [1, 18], [0, 18], [2, 18], [1, 19]],
    /* pos 5 */ [[2, 20], [2, 21], [1, 21], [0, 21], [0, 20], [1, 17], [1, 20]],
    /* pos 6 */ [[0, 22], [2, 23], [0, 23], [0, 22], [1, 22], [2, 22], [1, 23]],
    /* pos 7 */ [[2, 1], [2, 10], [0, 1], [0, 0], [1, 0], [2, 0], [1, 1]],
    /* pos 8 */ [[2, 2], [2, 3], [0, 4], [0, 3], [0, 2], [1, 2], [1, 3]],
    /* pos 9 */ [[2, 4], [2, 5], [1, 6], [0, 6], [0, 5], [1, 4], [1, 5]],
];

/// Which of the seven segments A..G are lit for each digit, as a bitmask
/// (bit 0 = A ... bit 6 = G).
pub const DIGIT_SEGS: [u8; 10] = [
    0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f,
];

/// The five indicator segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Indicator {
    Signal = 0,
    Bell = 1,
    Pm = 2,
    H24 = 3,
    Lap = 4,
}

/// Glass coordinates `(com, seg)` of the five indicators.
pub const INDICATORS: [[u8; 2]; 5] = [
    [0, 17], /* Signal */
    [0, 16], /* Bell   */
    [2, 17], /* PM     */
    [2, 16], /* 24H    */
    [1, 10], /* LAP    */
];
