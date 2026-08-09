//! The launcher app: a menu of the other apps.

use pluto_core::{DateTime, Event, Hardware, Lcd, Runtime};

use crate::conf::ConfApp;
use crate::menu::{render, Item, MenuState};
use crate::time::TimeApp;
use crate::AppSet;

/// The launcher menu, same layout as pluto's.
const ITEMS: [Item; 9] = [
    Item::Text(b" time"),
    Item::Text(b"ctdn"),
    Item::Text(b" alarm"),
    Item::Text(b"chro"),
    Item::Text(b" play"),
    Item::Text(b"compa"),
    Item::Text(b"speed"),
    Item::Text(b"conf"),
    Item::Text(b"   otp"),
];
const HEADER: &[u8] = b"la";
const HEADER_POS: u8 = 8;

/// The launcher app: a single-view menu.
#[derive(Clone)]
pub struct Launcher {
    view: usize,
    st: MenuState,
}

impl Launcher {
    pub fn new() -> Self {
        Launcher {
            view: 0,
            st: MenuState::new(),
        }
    }

    pub(crate) fn enter<H: Hardware>(&mut self, _time: DateTime, _hw: &mut H) {}

    pub(crate) fn leave<H: Hardware>(&mut self, _time: DateTime, _hw: &mut H) {}

    pub(crate) fn current_view(&self) -> usize {
        self.view
    }

    pub(crate) fn set_current_view(&mut self, view: usize) {
        self.view = view;
    }

    pub(crate) fn main<H: Hardware>(
        &mut self,
        event: Event,
        _time: DateTime,
        hw: &mut H,
        rt: &mut Runtime<H, AppSet>,
    ) {
        let n = ITEMS.len() as u8;
        match event {
            Event::KeyDown => self.st.down(n),
            Event::KeyUp => self.st.up(n),
            Event::KeyEnter => match self.st.item_current {
                0 => rt.launch(AppSet::Time(TimeApp::new())),
                7 => rt.launch(AppSet::Conf(ConfApp::new())),
                _ => {}
            },
            Event::KeyEnterLong => rt.launch(AppSet::Time(TimeApp::new())),
            _ => {}
        }
        let mut lcd = Lcd::new(hw);
        render(
            &mut lcd,
            &ITEMS[self.st.item_current as usize],
            0,
            HEADER,
            HEADER_POS,
        );
    }
}
