//! pluto-core: the portable pluto watch framework.
//!
//! A port of the pluto firmware (github.com/carrotIndustries/pluto-fw) to
//! Rust. Like the original, the code is split into a portable layer
//! (this crate) and a hardware-dependent layer ([`crate::hw::Hardware`]),
//! so the same apps run on the MSP430FR6972 firmware and in the browser
//! emulator.
//!
//! - [`display`]: the F-91W glass, in pluto's position-based coordinates.
//! - [`lcd`]: high-level drawing (integers, text, indicators).
//! - [`app`]: the app/view runtime and event model.
//! - [`input`]: button gesture recognition.
//! - [`time`]: civil date/time math.
//! - [`hw`]: the hardware interface (RTC, backlight, buzzer).

#![no_std]

pub mod app;
pub mod display;
pub mod hw;
pub mod input;
pub mod lcd;
mod letters;
pub mod time;

pub use app::{App, Event, Runtime, View};
pub use display::{DigitDisplay, Display, Indicator};
pub use hw::{Hardware, Timedate};
pub use input::{ButtonScanner, KeyId};
pub use lcd::Lcd;
pub use time::DateTime;
