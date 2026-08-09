//! Hardware features beyond the LCD: backlight and buzzer.

use crate::display::Display;

/// A single note of a [`Hardware::melody`]: a tone at `freq_hz` (0 = rest)
/// lasting `ms` milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub freq_hz: u32,
    pub ms: u32,
}

/// Maximum number of notes a [`Hardware::melody`] can play in one call.
pub const MAX_MELODY_NOTES: usize = 64;

/// The platform the face runs on: a [`Display`] plus non-display
/// peripherals.
///
/// Both the emulator (`pluto-emu`) and the real firmware
/// (`pluto-hw`) implement this trait, so a face behaves identically on
/// both: on the emulator `set_backlight`/the buzzer calls drive the SVG glow
/// and a WebAudio beep, on the hardware they drive the LED/piezo GPIOs.
///
/// `beep` has a default implementation (a short standard beep); implementors
/// only have to provide `beep_ms`. `melody` and `stop_melody` default to
/// no-op, so platforms without a programmable buzzer can ignore them. A face
/// that wants to play tones only links in the code it actually calls (the
/// framework is monomorphised and `lto = true`), so unused buzzer features
/// cost no code or flash space. See the README for a `melody` example.
pub trait Hardware: Display {
    /// Turn the backlight on or off (the runtime handles the ~3 s auto-off).
    fn set_backlight(&mut self, on: bool);
    /// Play a short standard beep (defaults to [`Hardware::beep_ms`] with
    /// 60 ms).
    fn beep(&mut self) {
        self.beep_ms(60);
    }
    /// Beep for `ms` milliseconds at the buzzer's default frequency.
    fn beep_ms(&mut self, ms: u32);
    /// Play a melody in the background. Notes play back-to-back; a note with
    /// `freq_hz == 0` is a rest of `ms` milliseconds. Defaults to no-op.
    fn melody(&mut self, notes: &[Note]) {
        let _ = notes;
    }
    /// Stop any melody currently playing via [`Hardware::melody`]. Defaults to
    /// no-op.
    fn stop_melody(&mut self) {}
}
