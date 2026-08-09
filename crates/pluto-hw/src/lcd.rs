//! MSP430FR6972 LCD_C driver.
//!
//! A faithful Rust port of the Pluto board's `target/hal/lcd.c`: the F-91W
//! glass is driven by the MSP430 LCD_C peripheral in 3-mux mode with a charge
//! pump. Faces draw in "glass" coordinates; every driver call is translated
//! to the LCD_C memory bit through the configurable `display_map`.
//!
//! WIP: the exact `tgt_lcd_map` (glass segment -> LCD_C (SEG, COM) routing)
//! is board specific. Until it is filled in from the Pluto board schematic,
//! `display_map/display_map.json` stays at the identity mapping, which is
//! correct for the emulator only. Regenerate it for real hardware with:
//!
//!   python3 tools/gen_display_map.py

use msp430fr6972::LCD_C;
use pluto_core::display_map;
use pluto_core::Display;

/// LCD_C SEG lines the Pluto board drives (from lcd.c `lcd_enable_seg` calls:
/// SEG0..SEG10 and SEG16..SEG28).
pub const SEG_PINS_LO: u16 = 0x07ff; /* SEG0..SEG10 -> LCDCPCTL0 */
pub const SEG_PINS_HI: u16 = 0x1fff; /* SEG16..SEG28 -> LCDCPCTL1 */

pub struct Lcd {
    lcd: LCD_C,
}

impl Lcd {
    pub fn init(lcd: LCD_C) -> Self {
        let mut l = Lcd { lcd };
        l.configure();
        l.clear_all();
        l
    }

    /// Mirror of pluto-fw `lcd_init()`: 3-mux, ACLK, charge pump, contrast 15.
    fn configure(&mut self) {
        // LCDCCTL0 = LCDDIV_21 | LCDPRE__8 | LCD3MUX (0x5821; LCDON set last).
        //   LCDDIV = 0b0101 (0x5000), LCDPRE = 8 (0x0800), LCDMX = 0b10 (0x20).
        self.lcd
            .lcdcctl0()
            .write(|w| unsafe { w.bits(0x5000 | 0x0800 | 0x0020) });

        // LCDCBLKCTL = LCDBLKPRE__16384 | LCDBLKMOD_1 (blink prescaler 0xe0).
        self.lcd
            .lcdcblkctl()
            .write(|w| unsafe { w.bits(0x00e0 | 0x0001) });

        // LCDCVCTL = LCDCPEN | VLCD_15 (charge pump on, contrast max).
        self.lcd
            .lcdcvctl()
            .write(|w| unsafe { w.bits(0x0001 | 0x1e00) });

        // Enable the SEG lines used on the Pluto board.
        self.lcd
            .lcdcpctl0()
            .write(|w| unsafe { w.bits(SEG_PINS_LO) });
        self.lcd
            .lcdcpctl1()
            .write(|w| unsafe { w.bits(SEG_PINS_HI) });

        // LCDCMEMCTL |= LCDCLRM | LCDCLRBM (clear display + blink memory).
        self.lcd
            .lcdcmemctl()
            .modify(|w| unsafe { w.bits(0x03) });

        // LCDCCTL0 |= LCDON.
        self.lcd.lcdcctl0().modify(|w| w.lcdon().set_bit());
    }

    /// Read one LCD_C memory byte (`LCDM1..LCDM24`, index 0..23 = SEG0..SEG23).
    fn read_mem(&self, seg: u8) -> u8 {
        match seg {
            0 => self.lcd.lcdm1().read().bits(),
            1 => self.lcd.lcdm2().read().bits(),
            2 => self.lcd.lcdm3().read().bits(),
            3 => self.lcd.lcdm4().read().bits(),
            4 => self.lcd.lcdm5().read().bits(),
            5 => self.lcd.lcdm6().read().bits(),
            6 => self.lcd.lcdm7().read().bits(),
            7 => self.lcd.lcdm8().read().bits(),
            8 => self.lcd.lcdm9().read().bits(),
            9 => self.lcd.lcdm10().read().bits(),
            10 => self.lcd.lcdm11().read().bits(),
            11 => self.lcd.lcdm12().read().bits(),
            12 => self.lcd.lcdm13().read().bits(),
            13 => self.lcd.lcdm14().read().bits(),
            14 => self.lcd.lcdm15().read().bits(),
            15 => self.lcd.lcdm16().read().bits(),
            16 => self.lcd.lcdm17().read().bits(),
            17 => self.lcd.lcdm18().read().bits(),
            18 => self.lcd.lcdm19().read().bits(),
            19 => self.lcd.lcdm20().read().bits(),
            20 => self.lcd.lcdm21().read().bits(),
            21 => self.lcd.lcdm22().read().bits(),
            22 => self.lcd.lcdm23().read().bits(),
            _ => self.lcd.lcdm24().read().bits(),
        }
    }

    fn write_mem(&self, seg: u8, value: u8) {
        let w = |r: &msp430fr6972::Reg<msp430fr6972::lcd_c::lcdm1::LCDM1_SPEC>, v: u8| unsafe {
            r.write(|w| w.bits(v))
        };
        match seg {
            0 => w(&self.lcd.lcdm1, value),
            1 => w(&self.lcd.lcdm2, value),
            2 => w(&self.lcd.lcdm3, value),
            3 => w(&self.lcd.lcdm4, value),
            4 => w(&self.lcd.lcdm5, value),
            5 => w(&self.lcd.lcdm6, value),
            6 => w(&self.lcd.lcdm7, value),
            7 => w(&self.lcd.lcdm8, value),
            8 => w(&self.lcd.lcdm9, value),
            9 => w(&self.lcd.lcdm10, value),
            10 => w(&self.lcd.lcdm11, value),
            11 => w(&self.lcd.lcdm12, value),
            12 => w(&self.lcd.lcdm13, value),
            13 => w(&self.lcd.lcdm14, value),
            14 => w(&self.lcd.lcdm15, value),
            15 => w(&self.lcd.lcdm16, value),
            16 => w(&self.lcd.lcdm17, value),
            17 => w(&self.lcd.lcdm18, value),
            18 => w(&self.lcd.lcdm19, value),
            19 => w(&self.lcd.lcdm20, value),
            20 => w(&self.lcd.lcdm21, value),
            21 => w(&self.lcd.lcdm22, value),
            22 => w(&self.lcd.lcdm23, value),
            _ => unsafe {
                self.lcd.lcdm24.write(|w| w.bits(value))
            },
        }
    }
}

impl Display for Lcd {
    fn clear_all(&mut self) {
        self.lcd
            .lcdcmemctl()
            .modify(|w| unsafe { w.bits(0x03) });
    }

    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        let Some(entry) = display_map::find_glass(com, seg) else {
            return;
        };
        // In 3-mux mode each LCDMEM byte holds one SEG line; the COM index is
        // the bit position within that byte.
        let mut v = self.read_mem(entry.lcd_seg);
        let bit = 1u8 << entry.lcd_com;
        if on {
            v |= bit;
        } else {
            v &= !bit;
        }
        self.write_mem(entry.lcd_seg, v);
    }
}
