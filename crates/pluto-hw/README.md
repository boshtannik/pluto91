# pluto-hw

Firmware for the Pluto board (MSP430FR6972, F-91W-style watch), built on the
same `pluto-core` + `pluto-faces` code as the browser emulator.

## Status: WIP scaffold

This crate is **not part of the workspace** and is **not built yet**. It is a
starting point for wiring the portable framework to the real MSP430FR6972
board; the pin assignments mirror the original pluto-fw firmware
(`target/hal`).

### Missing / TODO

- **Toolchain**: Rust for MSP430 needs a nightly compiler + the
  `msp430-none-elf` target + TI's `msp430-elf-gcc` linker (not installed on
  this machine). The `msp430`/`msp430-rt` 0.2.x ecosystem used by the
  `msp430fr6972` PAC historically builds with older nightlies (2020–2022);
  exact versions in `Cargo.toml` are unverified.
- **display_map**: the glass -> LCD_C routing (`tgt_lcd_map`) is board
  specific. `display_map/display_map.json` (repo root) is still the identity
  mapping, correct for the emulator only. Fill it in from the Pluto board
  schematic and regenerate with `tools/gen_display_map.py`.
- **RTC**: `main.rs` currently counts time from a fixed boot instant; wire the
  RTC_C (calendar mode, as in pluto-fw `rtc.c`) for real wall-clock time.
- **Buzzer**: `beep_ms` is a placeholder GPIO square wave; move to TA0
  (SMCLK) as in pluto-fw `beepled.c`.
- **LCD pin routing**: `lcd.rs` follows pluto-fw `lcd_init()` (3-mux, charge
  pump, SEG0..SEG10 + SEG16..SEG28) but is unverified against a real board.

## Build (once the toolchain is available)

```sh
cd crates/pluto-hw
cargo build --release
# flash with mspdebug (e.g. mspdebug -C mspdebug.cfg rf2500 'prog target/msp430-none-elf/release/pluto-hw')
```

`rust-toolchain.toml` and `.cargo/config.toml` already select the
`msp430-none-elf` target, `-Zbuild-std=core` and the `link.x` script; build
with a nightly that matches the `msp430 0.2` ecosystem.
