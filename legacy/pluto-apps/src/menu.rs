//! Shared menu state and rendering, mirroring `svc/menu.c`.

use pluto_core::{Hardware, Lcd};

/// Navigation state of a menu, pluto's `svc_menu_state_t`.
#[derive(Clone, Copy, Debug)]
pub struct MenuState {
    pub item_current: u8,
    pub adj_mode: bool,
    pub adj_digit: u8,
}

impl MenuState {
    pub const fn new() -> Self {
        MenuState {
            item_current: 0,
            adj_mode: false,
            adj_digit: 0,
        }
    }

    pub fn reset(&mut self) {
        self.item_current = 0;
        self.adj_mode = false;
        self.adj_digit = 0;
    }

    /// Move to the next item (pluto's `INC_MOD`).
    pub fn down(&mut self, n: u8) {
        if n > 0 {
            self.item_current = (self.item_current + 1) % n;
        }
    }

    /// Move to the previous item (pluto's `DEC_MOD`).
    pub fn up(&mut self, n: u8) {
        if n > 0 {
            self.item_current = if self.item_current == 0 {
                n - 1
            } else {
                self.item_current - 1
            };
        }
    }
}

/// A menu item: the subset of pluto's `svc_menu_item_t` the current apps use.
#[derive(Clone, Copy)]
pub enum Item {
    /// A plain text item; pressing Enter runs the app's handler.
    Text(&'static [u8]),
    /// A choice: the label at position 0, the current choice string at `pos`.
    Choice {
        text: &'static [u8],
        pos: u8,
        choices: &'static [&'static [u8]],
    },
}

impl Item {
    /// The item's label, drawn at digit position 0.
    pub fn text(&self) -> &'static [u8] {
        match self {
            Item::Text(t) => t,
            Item::Choice { text, .. } => text,
        }
    }
}

/// Draw one menu frame: the header on top, the current item (and its choice
/// value) at the bottom. `value` is the current value of a choice item.
pub fn render<H: Hardware>(
    lcd: &mut Lcd<H>,
    item: &Item,
    value: u8,
    header: &'static [u8],
    header_pos: u8,
) {
    lcd.clear();
    if !header.is_empty() {
        lcd.puts(header_pos, header);
    }
    match item {
        Item::Text(text) => lcd.puts(0, text),
        Item::Choice { text, pos, choices } => {
            lcd.puts(0, text);
            let idx = (value as usize) % choices.len();
            lcd.puts(*pos, choices[idx]);
        }
    }
}
