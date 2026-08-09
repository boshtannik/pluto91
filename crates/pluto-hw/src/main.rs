//! Pluto firmware for the MSP430FR6972 board.
//!
//! The same `pluto-core` + `pluto-faces` code as the emulator, running on real
//! hardware. The main loop follows the original pluto-fw `main.c`: it
//! initialises the clock and the LCD, polls the buttons and calls
//! `Watch::tick`. The button debouncing / gesture handling is done by the
//! portable [`pluto_core::input::ButtonScanner`] instead of timer ISRs.
//!
//! WIP: this crate is a scaffold and is *not* built yet (needs the nightly
//! MSP430 toolchain, see README.md). Pin assignments follow pluto-fw's
//! `target/hal`; the RTC_C integration and the TA0 buzzer are TODO.
#![no_std]
#![no_main]

mod lcd;

use msp430fr6972::Peripherals;
use pluto_core::face::ButtonId;
use pluto_core::time::DateTime;
use pluto_core::watch::Watch;
use pluto_core::{Display, Hardware};
use pluto_faces::Faces;

use lcd::Lcd;

/// Pluto board pins (from pluto-fw `target/hal`): Light = PJ.0, Mode = PJ.2,
/// Alarm = P9.4 (inputs with pull-down, active high). Buzzer = P7.3,
/// backlight LED = P1.0 (currently driven as plain GPIO; move to TA0 PWM).
const BTN_LIGHT_BIT: u8 = 0; /* PJ.0 */
const BTN_MODE_BIT: u8 = 2;  /* PJ.2 */
const BTN_ALARM_BIT: u8 = 4; /* P9.4 */
const BEEP_BIT: u8 = 3;      /* P7.3 */
const LIGHT_BIT: u8 = 0;     /* P1.0 */

/// Poll interval of the main loop, in ms (also advances the RTC).
const TICK_MS: u64 = 250;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// The LCD plus the non-display peripherals a face can use.
struct HwHardware {
    lcd: Lcd,
    port12: msp430fr6972::PORT_1_2,
    port7: msp430fr6972::PORT_7,
    port9: msp430fr6972::PORT_9,
    portj: msp430fr6972::PORT_J,
}

impl Display for HwHardware {
    fn clear_all(&mut self) {
        self.lcd.clear_all();
    }

    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        self.lcd.set_segment(com, seg, on);
    }
}

impl Hardware for HwHardware {
    fn set_backlight(&mut self, on: bool) {
        self.port12
            .p1out()
            .write(|w| unsafe { w.bits(set_bit(self.port12.p1out().read().bits(), LIGHT_BIT, on)) });
    }

    fn beep_ms(&mut self, ms: u32) {
        // TODO: drive the piezo with TA0 (SMCLK) at 2.4 kHz as in
        // `beepled.c`. For now a placeholder square wave on P7.3.
        let end = ms.saturating_mul(16_000);
        let mut i = 0u32;
        while i < end {
            self.port7.p7out().write(|w| unsafe {
                w.bits(set_bit(self.port7.p7out().read().bits(), BEEP_BIT, i % 4 < 2))
            });
            i += 1;
        }
    }
}

/// Clock tree from pluto-fw `clk_init()`: DCO = 16 MHz, SMCLK = 4 MHz,
/// MCLK = 8 MHz, LFXT crystal on PJ.4/PJ.5.
fn clk_init(p: &Peripherals) {
    // CSCTL0 = password CSKEY (0xA5), unlock.
    p.CS.csctl0().write(|w| unsafe { w.bits(0xa500) });
    // CSCTL1 = DCORSEL | DCOFSEL_4 (DCO ~16 MHz).
    p.CS.csctl1().write(|w| unsafe { w.bits(0x9002) });
    // CSCTL3 = DIVA__1 | DIVS__4 | DIVM__2.
    p.CS.csctl3().write(|w| unsafe { w.bits(0x0012) });
    // Lock access again.
    p.CS.csctl0().write(|w| unsafe { w.bits(0x0000) });
    // TODO: configure PJ.4/PJ.5 as LFXT crystal pins (FUNC1 | IN).
}

/// Configure the three buttons as inputs with pull-down (pluto-fw INPD).
fn button_init(p: &Peripherals) {
    let (pj, p9) = (&p.PORT_J, &p.PORT_9);
    for bit in [BTN_LIGHT_BIT, BTN_MODE_BIT] {
        pj.pjdir().write(|w| unsafe { w.bits(pj.pjdir().read().bits() & !(1 << bit)) });
        pj.pjren().write(|w| unsafe { w.bits(pj.pjren().read().bits() | (1 << bit)) });
        pj.pjout().write(|w| unsafe { w.bits(pj.pjout().read().bits() & !(1 << bit)) });
    }
    p9.p9dir().write(|w| unsafe { w.bits(p9.p9dir().read().bits() & !(1 << BTN_ALARM_BIT)) });
    p9.p9ren().write(|w| unsafe { w.bits(p9.p9ren().read().bits() | (1 << BTN_ALARM_BIT)) });
    p9.p9out().write(|w| unsafe { w.bits(p9.p9out().read().bits() & !(1 << BTN_ALARM_BIT)) });
}

/// True when the given button bit is pressed (active high).
fn button_down(hw: &HwHardware, id: ButtonId) -> bool {
    match id {
        ButtonId::Light => (hw.portj.pjin().read().bits() & (1 << BTN_LIGHT_BIT)) != 0,
        ButtonId::Mode => (hw.portj.pjin().read().bits() & (1 << BTN_MODE_BIT)) != 0,
        ButtonId::Alarm => (hw.port9.p9in().read().bits() & (1 << BTN_ALARM_BIT)) != 0,
    }
}

fn set_bit(v: u8, bit: u8, on: bool) -> u8 {
    if on {
        v | (1 << bit)
    } else {
        v & !(1 << bit)
    }
}

#[msp430_rt::entry]
fn main() -> ! {
    let p = Peripherals::take().unwrap();

    // Disable the default high-impedance mode of the FRAM port pins.
    p.PMM.pm5ctl0().write(|w| unsafe { w.bits(0) });

    clk_init(&p);
    button_init(&p);
    let lcd = Lcd::init(p.LCD_C);

    let mut hw = HwHardware {
        lcd,
        port12: p.PORT_1_2,
        port7: p.PORT_7,
        port9: p.PORT_9,
        portj: p.PORT_J,
    };
    hw.clear_all();

    let mut watch = Watch::<Faces>::new();

    // TODO: read the real time from the RTC_C (pluto-fw `rtc.c`). For now the
    // watch counts up from a fixed boot time (2026-01-01 12:00:00 UTC).
    let mut now_ms: u64 = 1_767_268_800_000;

    let ids = [ButtonId::Light, ButtonId::Mode, ButtonId::Alarm];
    loop {
        now_ms += TICK_MS;
        let now = DateTime::from_epoch_ms(now_ms);
        watch.tick(now, &mut hw);

        // The runtime classifies raw samples into gestures and chords.
        for id in ids {
            watch.button_raw(id, button_down(&hw, id), now, &mut hw);
        }
    }
}
