//! Ad-hoc: boot the port and dump what it draws. Run with
//! `cargo test -p pluto-apps -- --nocapture debug_boot`.

use pluto_core::display::Display;
use pluto_core::{DateTime, Event, Hardware, Runtime, Timedate};
use pluto_apps::time::TimeApp;
use pluto_apps::AppSet;

#[derive(Default)]
struct Fake {
    segs: std::collections::BTreeSet<(u8, u8)>,
}

impl Display for Fake {
    fn clear_all(&mut self) {
        self.segs.clear();
    }
    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        if on {
            self.segs.insert((com, seg));
        } else {
            self.segs.remove(&(com, seg));
        }
    }
}

impl Hardware for Fake {
    fn rtc_get(&mut self) -> Timedate {
        let t = DateTime::from_epoch_ms(1_750_000_000_000);
        Timedate { h: t.hour, m: t.minute, s: t.second, dow: ((t.weekday as u8) + 6) % 7, dom: t.day, month: t.month as u8, year: t.year }
    }
    fn rtc_set_time(&mut self, _t: &Timedate) {}
    fn rtc_set_date(&mut self, _t: &Timedate) {}
    fn backlight_set(&mut self, _on: bool) {}
    fn beep(&mut self, _f: u16) {}
    fn aux_timer_set(&mut self, _running: bool) {}
}

#[test]
fn debug_boot() {
    let time = DateTime::from_epoch_ms(1_750_000_000_000);
    let mut hw = Fake::default();
    let mut rt = Runtime::new();
    rt.boot(AppSet::Time(TimeApp::new()), AppSet::Launcher(pluto_apps::launcher::Launcher::new()), time, &mut hw);
    rt.process(Event::Tick, time, &mut hw);
    println!("LIT {:?}", hw.segs.iter().collect::<Vec<_>>());
    println!("time = {:02}:{:02}:{:02} dom={} dow={}", time.hour, time.minute, time.second, time.day, time.weekday as u8);
    assert_eq!(hw.segs.len(), 44); // known from the WASM dump; adjust if changed
}
