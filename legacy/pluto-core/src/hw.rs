//! Hardware interface, mirroring the pluto firmware's `hal.h`: everything a
//! portable app may touch beyond the LCD glass. Both the MSP430 firmware and
//! the browser emulator implement this trait.

use crate::display::Display;

/// A date/time snapshot as the pluto firmware uses it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Timedate {
    pub h: u8,
    pub m: u8,
    pub s: u8,
    pub dow: u8,
    pub dom: u8,
    pub month: u8,
    pub year: u16,
}

/// Hardware features a pluto app can use.
///
/// The [`Display`] supertrait is the glass itself; [`Hardware`] adds the
/// non-LCD peripherals. Apps drive the screen through an [`crate::lcd::Lcd`]
/// wrapper and the RTC/buzzer/backlight directly through this trait.
pub trait Hardware: Display {
    /// Read the current date and time from the real-time clock.
    fn rtc_get(&mut self) -> Timedate;
    /// Set the RTC's time-of-day portion (h/m/s).
    fn rtc_set_time(&mut self, t: &Timedate);
    /// Set the RTC's calendar portion (year/month/day/dow).
    fn rtc_set_date(&mut self, t: &Timedate);
    /// Turn the backlight on (`on`) or off. The runtime switches it off after
    /// ~3 s.
    fn backlight_set(&mut self, on: bool);
    /// Start a beep at `freq_hz` (0 = silent) until `aux_timer_set(false)`.
    fn beep(&mut self, freq_hz: u16);
    /// The auxiliary timer counts down and raises [`crate::app::Event::AuxTimer`]
    /// when it reaches zero. `running = true` starts (or restarts) it,
    /// `false` stops it.
    fn aux_timer_set(&mut self, running: bool);
}
