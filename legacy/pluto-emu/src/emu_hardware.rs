//! [`Hardware`] implementation that renders onto the F-91W SVG skin in the
//! browser and maps the backlight/buzzer onto the page's overlay and
//! WebAudio.

use pluto_core::{DateTime, Display, Hardware, Timedate};

extern "C" {
    fn js_clear();
    fn js_seg(com: u32, seg: u32, on: u32);
    fn js_backlight(on: u32);
    fn js_beep(freq: u32, ms: u32, delay_ms: u32);
    fn js_now() -> f64;
}

/// Stateless: all state lives in the DOM / the page. The RTC is the browser
/// clock (`js_now`).
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
    fn rtc_get(&mut self) -> Timedate {
        let t = DateTime::from_epoch_ms(now_ms());
        Timedate {
            h: t.hour,
            m: t.minute,
            s: t.second,
            // pluto numbers the weekdays 0 = Monday; our Weekday is 0 = Sunday.
            dow: ((t.weekday as u8) + 6) % 7,
            dom: t.day,
            month: t.month as u8,
            year: t.year,
        }
    }

    fn rtc_set_time(&mut self, _t: &Timedate) {}
    fn rtc_set_date(&mut self, _t: &Timedate) {}

    fn backlight_set(&mut self, on: bool) {
        unsafe { js_backlight(on as u32) };
    }

    fn beep(&mut self, freq_hz: u16) {
        if freq_hz > 0 {
            unsafe { js_beep(freq_hz as u32, 80, 0) };
        }
    }

    fn aux_timer_set(&mut self, _running: bool) {}
}

fn now_ms() -> u64 {
    unsafe { js_now() as u64 }
}
