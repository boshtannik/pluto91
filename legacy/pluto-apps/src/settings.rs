//! Shared watch settings, mirroring pluto's infomem-backed globals.

use core::cell::UnsafeCell;

pub const BASE_DEC: u8 = 0;
pub const BASE_HEX: u8 = 1;
pub const BASE_BIN: u8 = 2;

pub const LANG_EN: u8 = 0;
pub const LANG_DE: u8 = 1;
pub const LANG_FR: u8 = 2;

/// The number bases the time face can show the time in.
pub const BASE_CHOICES: &[&[u8]] = &[b"de", b"he", b"bi"];
/// The weekday languages.
pub const LANG_CHOICES: &[&[u8]] = &[b"en", b"de", b"fr"];

struct Settings {
    base: u8,
    lang: u8,
}

struct SettingsGlobal(UnsafeCell<Settings>);
// Single-threaded (browser WASM / one ISR context on the MSP430): the
// emulator and the firmware only ever touch these from one context.
unsafe impl Sync for SettingsGlobal {}

static SETTINGS: SettingsGlobal = SettingsGlobal(UnsafeCell::new(Settings {
    base: BASE_DEC,
    lang: LANG_EN,
}));

pub fn base() -> u8 {
    unsafe { (*SETTINGS.0.get()).base }
}

pub fn set_base(base: u8) {
    unsafe { (*SETTINGS.0.get()).base = base % 3 };
}

pub fn lang() -> u8 {
    unsafe { (*SETTINGS.0.get()).lang }
}

pub fn set_lang(lang: u8) {
    unsafe { (*SETTINGS.0.get()).lang = lang % 3 };
}
