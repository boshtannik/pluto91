//! The configuration app: a menu of settings.

use pluto_core::{DateTime, Event, Hardware, Lcd, Runtime};

use crate::menu::{render, Item, MenuState};
use crate::settings;
use crate::AppSet;

const HEADER: &[u8] = b"cf";
const HEADER_POS: u8 = 8;

/// The configuration menu.
const ITEMS: [Item; 2] = [
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

/// The configuration app: a single-view menu.
#[derive(Clone)]
pub struct ConfApp {
    view: usize,
    st: MenuState,
}

impl ConfApp {
    pub fn new() -> Self {
        ConfApp {
            view: 0,
            st: MenuState::new(),
        }
    }

    pub(crate) fn enter<H: Hardware>(&mut self, _time: DateTime, _hw: &mut H) {
        self.st.reset();
    }

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
                0 => settings::set_base(settings::base().wrapping_add(1)),
                1 => settings::set_lang(settings::lang().wrapping_add(1)),
                _ => {}
            },
            Event::KeyEnterLong => {
                rt.exit();
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
            &ITEMS[self.st.item_current as usize],
            value,
            HEADER,
            HEADER_POS,
        );
    }
}
