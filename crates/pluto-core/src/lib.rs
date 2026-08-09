//! Pluto watch framework.
//!
//! Portable, `no_std` core shared by the real firmware (`pluto-hw`) and the
//! browser emulator (`pluto-emu`). Faces implement [`face::Face`], the LCD
//! and other hardware is abstracted behind [`hardware::Hardware`] (a supertrait
//! of [`display::Display`]), and [`watch::Watch`] glues the two together and
//! handles mode cycling.
#![no_std]

pub mod display;
pub mod display_map;
pub mod face;
pub mod font;
pub mod hardware;
pub mod input;
pub mod letters;
pub mod time;
pub mod watch;

pub use display::{DigitDisplay, Display};
pub use face::{ButtonId, Face, FaceContext, GestureEvent, GestureKind};
pub use font::{Indicator, FONT};
pub use hardware::{Hardware, Note};
pub use time::{DateTime, Month, Weekday};
pub use watch::{FaceSet, Watch};
