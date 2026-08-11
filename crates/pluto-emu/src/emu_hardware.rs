//! [`Hardware`] implementation that renders onto the F-91W SVG skin in the
//! browser and maps the backlight/buzzer onto the page's overlay and
//! WebAudio. `js_beep(freq, ms, delay_ms)` schedules a tone in the future;
//! melodies are just a sequence of such calls with growing delays.

use pluto_core::{Display, Hardware, Note};

extern "C" {
    fn js_clear();
    fn js_seg(com: u32, seg: u32, on: u32);
    fn js_backlight(on: u32);
    fn js_beep(freq: u32, ms: u32, delay_ms: u32);
    fn js_stop_melody();
    fn js_set_time(ms: f64);
}

/// The emulator's default buzzer frequency (2.4 kHz).
const BUZZ_FREQ_HZ: u32 = 2400;

/// Stateless: all state lives in the DOM / the page.
pub struct EmuHardware;

impl Display for EmuHardware {
    fn clear_all(&mut self) {
        unsafe { js_clear() };
    }

    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        unsafe { js_seg(com as u32, seg as u32, on as u32) };
    }
}

impl Hardware for EmuHardware {
    fn set_backlight(&mut self, on: bool) {
        unsafe { js_backlight(on as u32) };
    }

    fn beep_ms(&mut self, ms: u32) {
        unsafe { js_beep(BUZZ_FREQ_HZ, ms, 0) };
    }

    fn melody(&mut self, notes: &[Note]) {
        let mut delay = 0u32;
        for note in notes {
            if note.freq_hz > 0 {
                unsafe { js_beep(note.freq_hz, note.ms, delay) };
            }
            delay = delay.saturating_add(note.ms);
        }
    }

    fn stop_melody(&mut self) {
        unsafe { js_stop_melody() };
    }

    fn set_rtc(&mut self, epoch_ms: u64) {
        // The page owns the clock (`js_now`): shifting its baseline makes the
        // emulator keep ticking from the newly set time.
        unsafe { js_set_time(epoch_ms as f64) };
    }
}
