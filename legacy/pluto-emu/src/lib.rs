//! Browser emulator for the pluto watch.
//!
//! The *same* `pluto-core` + `pluto-apps` code as the real firmware, compiled
//! to WASM and driven by the JavaScript page. `js_*` imports are provided by
//! the emulator page (`emulator/watch.html`).
#![no_std]

mod emu_hardware;

use core::cell::UnsafeCell;

use pluto_apps::{launcher, time, AppSet};
use pluto_core::input::{ButtonScanner, KeyId};
use pluto_core::{DateTime, Event, Runtime};

use emu_hardware::EmuHardware;

/// A `Sync` wrapper so the runtime can live in a `static`.
struct Global(UnsafeCell<Option<Runtime<EmuHardware, AppSet>>>);
unsafe impl Sync for Global {}

static RUNTIME: Global = Global(UnsafeCell::new(None));

/// Per-button gesture scanners, shared with the page.
struct Scanners(UnsafeCell<[ButtonScanner; 3]>);
unsafe impl Sync for Scanners {}

static SCANNERS: Scanners = Scanners(UnsafeCell::new([ButtonScanner::new(); 3]));

fn with_runtime<R>(f: impl FnOnce(&mut Option<Runtime<EmuHardware, AppSet>>) -> R) -> R {
    unsafe { f(&mut *RUNTIME.0.get()) }
}

/// Called once by the page after instantiating the module. Boots into the
/// time face, like the real firmware; `exit()` returns to the launcher.
#[no_mangle]
pub extern "C" fn pluto_init() {
    let time = DateTime::from_epoch_ms(now_ms());
    let mut hw = EmuHardware;
    with_runtime(|r| {
        *r = Some(Runtime::new());
        if let Some(rt) = r.as_mut() {
            rt.boot(
                AppSet::Time(time::TimeApp::new()),
                AppSet::Launcher(launcher::Launcher::new()),
                time,
                &mut hw,
            );
        }
    });
}

/// Called periodically by the page; `ms` is milliseconds since the Unix epoch
/// (used as the watch's wall clock).
#[no_mangle]
pub extern "C" fn pluto_tick(ms: f64) {
    let time = DateTime::from_epoch_ms(ms as u64);
    let mut hw = EmuHardware;
    with_runtime(|r| {
        if let Some(rt) = r.as_mut() {
            rt.process(Event::Tick, time, &mut hw);
        }
    });
}

/// Called by the page on button state changes. IDs: 0 = Light (Up),
/// 1 = Mode (Down), 2 = Alarm (Enter), matching pluto's button roles. `down`
/// is 1 while the button is pressed, 0 on release. The page can keep sending
/// `down = 1` to drive hold auto-repeat.
#[no_mangle]
pub extern "C" fn pluto_button(id: u32, down: u32) {
    let key = match id {
        0 => KeyId::Up,
        1 => KeyId::Down,
        _ => KeyId::Enter,
    };
    let index = (id as usize).min(2);
    let event = unsafe { (*SCANNERS.0.get())[index].sample(key, down != 0, now_ms()) };
    if let Some(event) = event {
        let time = DateTime::from_epoch_ms(now_ms());
        let mut hw = EmuHardware;
        with_runtime(|r| {
            if let Some(rt) = r.as_mut() {
                rt.process(event, time, &mut hw);
            }
        });
    }
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
