//! The time app: the main clock face and its configuration menu.

use pluto_core::{DateTime, Event, Hardware, Indicator, Lcd, Runtime, Timedate, View};

use crate::menu::{render, Item, MenuState};
use crate::settings::{self, BASE_BIN, BASE_HEX};
use crate::AppSet;

const HEADER: &[u8] = b"cf";
const HEADER_POS: u8 = 8;

/// The time app's configuration menu, mirroring the first items of pluto's
/// `time` menu.
const MENU_ITEMS: [Item; 2] = [
    Item::Choice {
        text: b"base",
        pos: 4,
        choices: settings::BASE_CHOICES,
    },
    Item::Choice {
        text: b"lang",
        pos: 4,
        choices: settings::LANG_CHOICES,
    },
];

/// A `Timedate` that differs from any real value, to force a full redraw.
const FORCE_UPDATE: Timedate = Timedate {
    h: 0xff,
    m: 0xff,
    s: 0xff,
    dow: 0xff,
    dom: 0xff,
    month: 0xff,
    year: 0xffff,
};

/// Weekday string for pluto's `dow` (0 = Monday).
fn dow_string(dow: u8, lang: u8) -> &'static [u8] {
    const EN: [&[u8]; 7] = [b"mo", b"tu", b"we", b"th", b"fr", b"sa", b"su"];
    const DE: [&[u8]; 7] = [b"mo", b"di", b"mi", b"do", b"fr", b"sa", b"so"];
    const FR: [&[u8]; 7] = [b"lu", b"ma", b"me", b"je", b"ve", b"sa", b"di"];
    let i = (dow % 7) as usize;
    match lang {
        settings::LANG_DE => DE[i],
        settings::LANG_FR => FR[i],
        _ => EN[i],
    }
}

/// View 0: the clock face (pluto's `common/app/time/display.c`).
#[derive(Clone)]
pub struct TimeDisplay {
    needs_clear: bool,
    display_date: bool,
    td_last: Timedate,
}

impl TimeDisplay {
    fn new() -> Self {
        TimeDisplay {
            needs_clear: true,
            display_date: false,
            td_last: Timedate::default(),
        }
    }
}

impl<H: Hardware> View<H, AppSet> for TimeDisplay {
    fn enter(&mut self, _time: DateTime, _hw: &mut H) {
        self.needs_clear = true;
    }

    fn main(&mut self, event: Event, _time: DateTime, hw: &mut H, rt: &mut Runtime<H, AppSet>) {
        let td = hw.rtc_get();
        if self.needs_clear {
            self.td_last = FORCE_UPDATE;
        }
        match event {
            Event::KeyEnterLong => {
                rt.set_view(1);
                return;
            }
            Event::KeyDown => {
                self.display_date = !self.display_date;
                self.needs_clear = true;
                self.td_last = FORCE_UPDATE;
            }
            Event::KeyUp => {
                rt.exit();
                return;
            }
            _ => {}
        }
        if self.needs_clear {
            hw.clear_all();
        }
        let mut lcd = Lcd::new(hw);
        if self.display_date {
            lcd.indicator(Indicator::Colon, false);
            lcd.puti(0, 4, td.year as u32);
            lcd.puti(4, 2, td.month as u32);
            lcd.puti(6, 2, td.dom as u32);
        } else {
            match settings::base() {
                BASE_HEX => {
                    lcd.indicator(Indicator::Colon, true);
                    if td.h != self.td_last.h {
                        lcd.putix(0, 2, td.h as u32);
                    }
                    if td.m != self.td_last.m {
                        lcd.putix(2, 2, td.m as u32);
                    }
                    if td.s != self.td_last.s {
                        lcd.putix(4, 2, td.s as u32);
                    }
                    if td.dom != self.td_last.dom {
                        lcd.puti(6, 2, td.dom as u32);
                    }
                }
                BASE_BIN => {
                    // Binary mode on the F-91W uses the signal-bar area; the
                    // skin groups all five bars behind one segment.
                    lcd.indicator(Indicator::Bars, true);
                    if td.dom != self.td_last.dom {
                        lcd.puti(6, 2, td.dom as u32);
                    }
                }
                _ => {
                    // BASE_DEC
                    lcd.indicator(Indicator::Colon, true);
                    if td.h != self.td_last.h {
                        lcd.puti(0, 2, td.h as u32);
                    }
                    if td.m != self.td_last.m {
                        lcd.puti(2, 2, td.m as u32);
                    }
                    if td.s != self.td_last.s {
                        lcd.puti(4, 2, td.s as u32);
                    }
                    if td.dom != self.td_last.dom {
                        lcd.puti(6, 2, td.dom as u32);
                    }
                }
            }
        }
        if td.dow != self.td_last.dow {
            lcd.puts(8, dow_string(td.dow, settings::lang()));
        }
        self.td_last = td;
        self.needs_clear = false;
    }
}

/// View 1: the time app's configuration menu.
#[derive(Clone)]
pub struct TimeMenu {
    st: MenuState,
}

impl<H: Hardware> View<H, AppSet> for TimeMenu {
    fn enter(&mut self, _time: DateTime, _hw: &mut H) {
        self.st.reset();
    }

    fn main(&mut self, event: Event, _time: DateTime, hw: &mut H, rt: &mut Runtime<H, AppSet>) {
        let n = MENU_ITEMS.len() as u8;
        match event {
            Event::KeyDown => self.st.down(n),
            Event::KeyUp => self.st.up(n),
            Event::KeyEnter => match self.st.item_current {
                0 => settings::set_base(settings::base().wrapping_add(1)),
                1 => settings::set_lang(settings::lang().wrapping_add(1)),
                _ => {}
            },
            Event::KeyEnterLong => {
                rt.set_view(0);
                return;
            }
            _ => {}
        }
        let value = match self.st.item_current {
            0 => settings::base(),
            _ => settings::lang(),
        };
        let mut lcd = Lcd::new(hw);
        render(
            &mut lcd,
            &MENU_ITEMS[self.st.item_current as usize],
            value,
            HEADER,
            HEADER_POS,
        );
    }
}

/// The time app: view 0 = clock face, view 1 = configuration menu.
#[derive(Clone)]
pub struct TimeApp {
    view: usize,
    display: TimeDisplay,
    menu: TimeMenu,
}

impl TimeApp {
    pub fn new() -> Self {
        TimeApp {
            view: 0,
            display: TimeDisplay::new(),
            menu: TimeMenu {
                st: MenuState::new(),
            },
        }
    }

    pub(crate) fn enter<H: Hardware>(&mut self, time: DateTime, hw: &mut H) {
        match self.view {
            0 => self.display.enter(time, hw),
            _ => self.menu.enter(time, hw),
        }
    }

    pub(crate) fn leave<H: Hardware>(&mut self, time: DateTime, hw: &mut H) {
        match self.view {
            0 => self.display.leave(time, hw),
            _ => self.menu.leave(time, hw),
        }
    }

    pub(crate) fn current_view(&self) -> usize {
        self.view
    }

    pub(crate) fn set_current_view(&mut self, view: usize) {
        self.view = view;
    }

    pub(crate) fn main<H: Hardware>(
        &mut self,
        event: Event,
        time: DateTime,
        hw: &mut H,
        rt: &mut Runtime<H, AppSet>,
    ) {
        match self.view {
            0 => self.display.main(event, time, hw, rt),
            _ => self.menu.main(event, time, hw, rt),
        }
    }
}
