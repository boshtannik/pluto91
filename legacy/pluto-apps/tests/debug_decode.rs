//! Ad-hoc: decode what the port draws per digit position.

use pluto_core::display::Display;
use pluto_core::{DateTime, Event, Hardware, Runtime, Timedate};
use pluto_apps::time::TimeApp;
use pluto_apps::AppSet;

#[derive(Default)]
struct Fake {
    segs: std::collections::BTreeSet<(u8, u8)>,
}
impl Display for Fake {
    fn clear_all(&mut self) { self.segs.clear(); }
    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        if on { self.segs.insert((com, seg)); } else { self.segs.remove(&(com, seg)); }
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

const FONT: [[[i8; 2]; 7]; 8] = [
    [[1, 18], [2, 19], [0, 19], [1, 18], [0, 18], [2, 18], [1, 19]],
    [[2, 20], [2, 21], [1, 21], [0, 21], [0, 20], [1, 17], [1, 20]],
    [[0, 22], [2, 23], [0, 23], [0, 22], [1, 22], [2, 22], [1, 23]],
    [[2, 1], [2, 10], [0, 1], [0, 0], [1, 0], [2, 0], [1, 1]],
    [[2, 2], [2, 3], [0, 4], [0, 3], [0, 2], [1, 2], [1, 3]],
    [[2, 4], [2, 5], [1, 6], [0, 6], [0, 5], [1, 4], [1, 5]],
    [[1, 9], [0, 9], [2, 9], [1, 9], [0, 10], [-1, -1], [1, 9]],
    [[0, 7], [1, 7], [2, 7], [2, 6], [2, 8], [0, 8], [1, 8]],
];

#[test]
fn debug_decode() {
    let time = DateTime::from_epoch_ms(1_750_000_000_000);
    let mut hw = Fake::default();
    let mut rt = Runtime::new();
    rt.boot(AppSet::Time(TimeApp::new()), AppSet::Launcher(pluto_apps::launcher::Launcher::new()), time, &mut hw);
    rt.process(Event::Tick, time, &mut hw);

    let names = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];
    for pos in 0..8 {
        let mut mask = 0u8;
        for (i, seg) in FONT[pos].iter().enumerate() {
            if seg[0] >= 0 && seg[1] >= 0 && hw.segs.contains(&(seg[0] as u8, seg[1] as u8)) {
                mask |= 1 << i;
            }
        }
        let mut lit = String::new();
        for i in 0..7 {
            if mask & (1 << i) != 0 { lit.push(names[i]); }
        }
        println!("pos {} (h,m,s,dom digits): segments lit = {}", pos, lit);
    }
}
