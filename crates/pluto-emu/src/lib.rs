//! Browser emulator for the Pluto watch.
//!
//! The *same* `pluto-core` + `pluto-faces` code as the real firmware, compiled
//! to WASM and driven by the JavaScript page. `js_*` imports are provided by
//! the emulator page (`emulator/build/watch.html`).
#![no_std]

mod emu_hardware;

use core::cell::UnsafeCell;

use pluto_faces::Faces;
use pluto_core::face::ButtonId;
use pluto_core::watch::Watch;
use pluto_core::{DateTime, Display};

use emu_hardware::EmuHardware;

/// A `Sync` wrapper so the runtime can live in a `static`.
struct Global(UnsafeCell<Option<Watch<Faces>>>);
unsafe impl Sync for Global {}

static WATCH: Global = Global(UnsafeCell::new(None));

fn with_watch<R>(f: impl FnOnce(&mut Option<Watch<Faces>>) -> R) -> R {
    unsafe { f(&mut *WATCH.0.get()) }
}

/// Called once by the page after instantiating the module.
#[no_mangle]
pub extern "C" fn pluto_init() {
    with_watch(|w| {
        *w = Some(Watch::new());
    });
    let mut hw = EmuHardware;
    hw.clear_all();
}

/// Number of faces compiled into this build (depends on `faces.toml` in
/// `pluto-faces`). Exported so the emulator page / tests can check which
/// faces the wasm actually contains.
#[no_mangle]
pub extern "C" fn pluto_face_count() -> u32 {
    <Faces as pluto_core::watch::FaceSet>::LEN as u32
}

/// Set the idle time (seconds) after which the watch returns to the clock
/// face by itself; `0` disables the auto-return. Exposed for the emulator
/// page and tests.
#[no_mangle]
pub extern "C" fn pluto_set_auto_home(secs: u32) {
    with_watch(|w| {
        if let Some(watch) = w.as_mut() {
            watch.set_auto_home_secs(secs as u64);
        }
    });
}

/// Called periodically by the page; `ms` is milliseconds since the Unix epoch
/// (used as the watch's wall clock).
#[no_mangle]
pub extern "C" fn pluto_tick(ms: f64) {
    let time = DateTime::from_epoch_ms(ms as u64);
    let mut hw = EmuHardware;
    with_watch(|w| {
        if let Some(watch) = w.as_mut() {
            watch.tick(time, &mut hw);
        }
    });
}

/// Called by the page on button state changes. IDs: 0 = Light, 1 = Mode,
/// 2 = Alarm. `down` is 1 while the button is pressed, 0 on release. The
/// page can keep sending `down = 1` to drive hold auto-repeat. Chords (two
/// buttons down together) are recognised by the runtime.
#[no_mangle]
pub extern "C" fn pluto_button(id: u32, down: u32) {
    let button = match id {
        0 => ButtonId::Light,
        1 => ButtonId::Mode,
        _ => ButtonId::Alarm,
    };
    let time = DateTime::from_epoch_ms(now_ms());
    let mut hw = EmuHardware;
    with_watch(|w| {
        if let Some(watch) = w.as_mut() {
            watch.button_raw(button, down != 0, time, &mut hw);
        }
    });
}

extern "C" {
    fn js_now() -> f64;
}

fn now_ms() -> u64 {
    unsafe { js_now() as u64 }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    extern "C" {
        fn js_panic();
    }
    unsafe { js_panic() };
    loop {}
}
