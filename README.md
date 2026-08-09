# Pluto

**English** · [Русский](README.ru.md) · [User manual](MANUAL.md)

Firmware for a Casio F-91W-style wristwatch written in Rust.

The same logic (the `pluto-core` + `pluto-faces` crates) runs in two places:

- **browser emulator** (`pluto-emu` → WASM) — for development and debugging without hardware;
- **real board** (`pluto-hw` → MSP430) — firmware for a replacement F-91W board.

```
                 pluto-faces (faces: SimpleClock, Alarm)
                        │  implement the Face trait
                 pluto-core (framework: Watch, Face, buttons, display)
                       ╱                 ╲
           pluto-emu (WASM)          pluto-hw (MSP430)
        browser emulator              real board
```

---

## Table of contents

- [The board](#the-board)
- [Quick start: emulator](#quick-start-emulator)
- [Building and flashing the real board](#building-and-flashing-the-real-board)
- [How the framework works](#how-the-framework-works)
  - [Crates](#crates)
  - [The F-91W glass and segment coordinates](#the-f-91w-glass-and-segment-coordinates)
  - [The `Watch` runtime](#the-watch-runtime)
  - [Buttons and gestures](#buttons-and-gestures)
  - [Time](#time)
  - [display_map and letters (data generation)](#display_map-and-letters-data-generation)
- [How to write your own face](#how-to-write-your-own-face)
- [Tests](#tests)
- [legacy/](#legacy)

---

## The board

The firmware targets the **Casio F-91W replacement board "Pluto"** (originating
from the pluto-fw project), built around the **MSP430FR6972** microcontroller
(Texas Instruments, FRAM, ~64 KB of memory). The watch keeps the original F-91W
glass — a liquid-crystal display with 10 seven-segment positions and indicators.

Board pin-out (`crates/pluto-hw/src/main.rs`, following the pluto-fw sources under `target/hal`):

| Peripheral   | Pin    | Note                                   |
|--------------|--------|----------------------------------------|
| Light button | `PJ.0` | input with pull-down, active-high      |
| Mode button  | `PJ.2` | input with pull-down, active-high      |
| Alarm button | `P9.4` | input with pull-down, active-high      |
| Buzzer       | `P7.3` | square-wave stub; TODO: move to TA0    |
| Backlight    | `P1.0` | plain GPIO; TODO: PWM via TA0          |
| Display      | LCD_C  | 3-mux, charge pump, contrast 15        |

The watch ticks every **250 ms** (`TICK_MS`); the emulator ticks at the same rate.

> **`pluto-hw` status — WIP.** The crate is not part of the workspace and has
> not been built on this machine yet: it needs nightly Rust with the
> `msp430-none-elf` target and the TI linker `msp430-elf-gcc`. The RTC and
> buzzer drivers, as well as the `display_map` correspondence to the real board,
> have not been verified yet. See
> [`crates/pluto-hw/README.md`](crates/pluto-hw/README.md) for details.

---

## Quick start: emulator

For face development the emulator is enough — it runs the very same logic
(compiled to WASM) that goes to the board.

```sh
# 1. WASM target (once)
rustup target add wasm32-unknown-unknown

# 2. Build the emulator (WASM + page)
make -C emulator

# 3. Run
python3 -m http.server -d emulator/build
# open http://localhost:8000/watch.html
```

Or use the convenience script:

```sh
./run.sh   # builds the emulator, starts a server, opens watch.html in the browser
```

Controls:

- **on-page buttons** of the watch case: Light / Mode / Alarm;
- **keyboard**: `W` = Light, `S` = Mode, `D` = Alarm (or arrows ↑ / ↓ / →);
- **hold**: hold a button with the mouse — the emulator sends repeats itself;
  on the keyboard repeats depend on the OS key repeat.

---

## Building and flashing the real board

The `pluto-hw` crate is built standalone (not from the workspace root) because
it needs a nightly MSP430 toolchain.

```sh
# Install the toolchain (see crates/pluto-hw/rust-toolchain.toml):
#   nightly + rust-src + the msp430-none-elf target
# And the TI linker: msp430-elf-gcc
rustup target add --toolchain nightly msp430-none-elf
rustup component add --toolchain nightly rust-src

cd crates/pluto-hw
cargo build --release

# Flash via mspdebug (example for the rf2500 / MSP-FET debugger):
mspdebug rf2500 'prog target/msp430-none-elf/release/pluto-hw'
```

The build uses `-Zbuild-std=core`, the `link.x` linker script and the memory
layout from `memory.x` (RAM 2 KB @ `0x1C00`, ROM ~46.8 KB @ `0x4400`).

**Important before flashing a real board** (all TODO at the moment):

1. `display_map/display_map.json` — fill in the "glass → LCD_C" mapping from
   the board schematic and regenerate it (`python3 tools/gen_display_map.py`).
2. Hook up the **RTC_C** for real time (currently time is counted from a fixed
   boot moment).
3. Move the **buzzer** from the GPIO stub to the TA0 timer (SMCLK).
4. Verify the LCD pin-out (`lcd.rs`) on the real board.

---

## How the framework works

### Crates

| Crate        | Path                  | What it is |
|--------------|-----------------------|------------|
| `pluto-core` | `crates/pluto-core`   | The framework: `no_std`, no dependencies. Traits, runtime, gestures, display, time |
| `pluto-faces`| `crates/pluto-faces`  | The set of faces (watch programs): `SimpleClock`, `Alarm`; the `Faces` enum |
| `pluto-emu`  | `crates/pluto-emu`    | WASM bridge: `pluto_init` / `pluto_tick` / `pluto_button` + `js_*` imports |
| `pluto-hw`   | `crates/pluto-hw`     | MSP430 firmware: main loop + LCD_C driver (standalone crate) |

`pluto-core` is the foundation. Its public API:

```rust
pub mod display;      // Display (glass) + DigitDisplay (convenient renderers)
pub mod display_map;  // generation: display_map/display_map.rs
pub mod face;         // trait Face, FaceContext, gestures, chords, AlarmAction
pub mod font;         // FONT (segment positions), DIGIT_SEGS, INDICATORS
pub mod hardware;     // Hardware: backlight, buzzer, melodies
pub mod input;        // ButtonScanner: gesture recognition
pub mod letters;      // generation: display_map/letters.rs
pub mod time;         // DateTime, Weekday, Month
pub mod watch;        // Watch<F>: runtime + FaceSet
```

The key idea: **faces are plain structs** implementing `trait Face`. They know
nothing about the hardware or WASM; all access to the display and effects goes
through `Hardware` (narrowed down to a concrete platform). That is why the same
face works identically in the emulator and on the board.

### The F-91W glass and segment coordinates

All coordinates are **glass** `(com, seg)` — the same ones used in the
emulator SVG skin and in the `FONT` table (`crates/pluto-core/src/font.rs`).
Each driver (emulator, LCD_C) maps them to its own bits via `display_map`.

Digit positions (0-indexed):

```
  0  1       2  3        4  5 : 6  7 : 8  9
weekday    day          HH     MM     SS
```

- **0–1** — weekday letters / mode labels (SU, MO, TU, …, AL, ST).
  Drawn with `set_char`, not `set_digit`: the segment sets for letters live in
  `letters/letters.json`.
- **2–3** — day of month.
- **4–9** — HH:MM:SS (no leading zero on the hour — the face decides).
- **Indicators** — `Signal`, `Bell`, `Pm`, `H24`, `Lap`
  (`font::Indicator`, coordinates in `INDICATORS`).

The F-91W glass shares some segments (e.g. the day of month cannot draw proper
"tens"), so in the `FONT` table such cells are marked `(-1, -1)` and skipped.

### The `Watch` runtime

`Watch<F: FaceSet>` owns **all** faces at once (the `F::Faces` array).
Pressing Mode only switches the active face while each face keeps its state —
so an alarm set in `Alarm` is not lost when switching to `SimpleClock` and back.

Ticking (`Watch::tick`, every 250 ms):

1. auto-off of the backlight (~3 s after pressing Light);
2. hourly signal, if `chime` is enabled;
3. `background_tick()` for **every** face — background work that must run
   regardless of the visible face (e.g. the alarm firing). A face may return
   `true` to ask the runtime to switch to it;
4. `tick()` of only the active face (it draws the screen).

Faces redraw the entire screen on each tick: segment writes are idempotent,
so redrawing is cheap and safe.

### Buttons and gestures

Layout: **Light**, **Mode**, **Alarm**. Handling in the runtime:

- **Mode** — handled entirely by the runtime: a quick `Press` switches faces.
- **Light** — `Hold` turns on the backlight (with auto-off); all gestures are
  also delivered to the active face.
- **Alarm** — the face gets it first; if the face does not "eat" the press and
  it is a `Press`, the runtime performs the face's global action
  `alarm_action()` (12/24-hour format or hourly-signal toggle).

Gestures are recognized by `ButtonScanner` (`crates/pluto-core/src/input.rs`):

| Gesture   | Condition                                        |
|-----------|--------------------------------------------------|
| `Press`   | quick tap — fires **on release**                 |
| `Double`  | two taps within 400 ms (`DOUBLE_CLICK_MS`)       |
| `Hold`    | hold > 750 ms (`HOLD_DELAY_MS`), auto-repeat every 250 ms (`REPEAT_MS`); the first repeat also counts as the hold press |
| chord     | two buttons pressed at once → `ChordEvent` on releasing both; press/hold of the chorded buttons are suppressed |

Gesture types live in `pluto-core::face`:

```rust
GestureEvent { button: ButtonId, kind: GestureKind } // Light | Mode | Alarm
GestureKind  ::= Press | Hold | Double
ChordEvent   { first: ButtonId, second: ButtonId }
AlarmAction  ::= H24Toggle | ChimeToggle
```

### Time

`time::DateTime` is built from Unix-epoch milliseconds
(`DateTime::from_epoch_ms`) — via Howard Hinnant's algorithm. Fields:

```rust
DateTime { secs: u64, ms: u16, year: u16, month: Month,
           day: u8, weekday: Weekday, hour: u8, minute: u8, second: u8 }
```

The platform supplies the time itself: the emulator uses the browser's real
clock, the board uses the RTC (a stub for now). Faces never set the time; they
only read it from `FaceContext`.

### display_map and letters (data generation)

- **`display_map/`** — the "glass segment → LCD bit" mapping.
  The single source of truth is `display_map.json`; the Rust table
  `display_map.rs` is generated by `tools/gen_display_map.py`. It is currently
  an identity mapping (correct for the emulator); real-board owners edit the
  JSON for their own wiring.
- **`letters/`** — the segment sets for weekday letters.
  Edited in `letters.json` (visually — `emulator/letters.html`), compiled by
  `tools/gen_letters.py` → `display_map/letters.rs`.

---

## How to write your own face

A face is a struct plus a `trait Face` implementation. Step by step:

### 1. Create a module

`crates/pluto-faces/src/my_face.rs`:

```rust
use pluto_core::face::{ButtonId, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::{DigitDisplay, Hardware};

/// My face. Keep any state that must survive Mode switches right here
/// (as a struct field).
#[derive(Clone, Copy, Default)]
pub struct MyFace {
    count: u8, // example: own state
}
```

### 2. Implement `Face`

```rust
impl Face for MyFace {
    // Called once when the face becomes active.
    fn init(&mut self, _ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.clear_all();
    }

    // Periodic tick of the active face: this is where we draw the screen.
    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.set_digit(4, ctx.time.hour / 10);
        hw.set_digit(5, ctx.time.hour % 10);
        hw.set_digit(6, ctx.time.minute / 10);
        hw.set_digit(7, ctx.time.minute % 10);
        // ...
    }

    // Background tick: also runs while the face is not active.
    // Return true to ask the runtime to switch to this face.
    fn background_tick(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    // Button handling. Return true if the gesture was fully handled —
    // then the runtime will NOT perform the global Alarm-button action.
    fn button(&mut self, event: GestureEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        match event {
            GestureEvent { button: ButtonId::Alarm, kind: GestureKind::Press } => {
                self.count = self.count.wrapping_add(1);
                true // ate the press
            }
            _ => false,
        }
    }

    // Chord handling (two buttons at once).
    fn chord(&mut self, event: pluto_core::ChordEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    // Global Alarm-button action when the face did not eat the press.
    fn alarm_action(&self) -> pluto_core::face::AlarmAction {
        pluto_core::face::AlarmAction::H24Toggle
    }
}
```

### 3. Register the face

In `crates/pluto-faces/src/lib.rs`:

1. `mod my_face;` and `pub use my_face::MyFace;`
2. add a variant to `enum Faces { SimpleClock(SimpleClock), Alarm(Alarm), MyFace(MyFace) }`;
3. add the delegation to every method of `impl Face for Faces`;
4. add an instance to `static ALL_FACES` (order = the Mode cycle order).

### 4. Check

```sh
cargo build                     # workspace (core + faces)
cargo test                      # unit tests
make -C emulator && node tools/emu_test.mjs   # emulator integration checks
```

Useful tricks:

- **Drawing digits** — `set_digit(pos, d)` / `clear_digit(pos)`
  (`DigitDisplay`). Don't forget to blank unused positions.
- **Letters/labels** — `set_char(pos, b'A')` (positions 0–1).
- **Indicators** — `set_indicator(Indicator::Bell, true)`.
- **Buzzer** — `hw.beep()` (short), `hw.beep_ms(ms)`, `hw.melody(&notes)`,
  `hw.stop_melody()` (`Note { freq_hz, ms }`, up to `MAX_MELODY_NOTES` notes).
- **Blinking** — base it on `ctx.time.ms`: the 250 ms interval conveniently
  matches the tick; see the blink example in `Alarm` (`Alarm::draw_edit`).
- **Long press action** — `GestureKind::Hold` (auto-repeat),
  `Double` for a quick double tap.

---

## Tests

- `cargo test` — `pluto-core` unit tests (the `ButtonScanner` gestures, time,
  letters in `tests/letters.rs`).
- `node tools/emu_test.mjs` — emulator integration checks (58 of them): they
  build the WASM (`make -C emulator`), run press/tick scenarios and verify the
  lit segments, the backlight and the buzzer.

---

## legacy/

The old generation of the framework (the "apps" model: launcher/menu/settings,
its own `pluto-core`/`pluto-emu` versions) is kept for reference. Active
development happens in `crates/`, on the **faces** model.
