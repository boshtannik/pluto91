//! Weekday-letter segment tables (data-driven).
//!
//! Which segments light for each character is defined in
//! `letters/letters.json` (edit visually with `emulator/letters.html`) and
//! compiled into Rust by `tools/gen_letters.py`. The generated file is
//! `display_map/letters.rs`.

include!("../../../display_map/letters.rs");
