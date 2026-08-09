# Pluto

[English](README.md) · **Русский** · [Инструкция пользователя](MANUAL.ru.md)

Прошивка для наручных часов в стиле Casio F-91W, написанная на Rust.

Одна и та же логика (крейты `pluto-core` + `pluto-faces`) работает в двух местах:

- **браузерный эмулятор** (`pluto-emu` → WASM) — для разработки и отладки без железа;
- **реальная плата** (`pluto-hw` → MSP430) — прошивка для заменяющей платы F-91W.

```
                 pluto-faces (фейсы: задаются в faces.toml)
                        │  реализуют trait Face
                 pluto-core (фреймворк: Watch, Face, кнопки, дисплей)
                       ╱                 ╲
           pluto-emu (WASM)          pluto-hw (MSP430)
        эмулятор в браузере          реальная плата
```

---

## Оглавление

- [Плата](#плата)
- [Быстрый старт: эмулятор](#быстрый-старт-эмулятор)
- [Сборка и прошивка реальной платы](#сборка-и-прошивка-реальной-платы)
- [Как устроен фреймворк](#как-устроен-фреймворк)
  - [Крейты](#крейты)
  - [Стекло F-91W и координаты сегментов](#стекло-f-91w-и-координаты-сегментов)
  - [Runtime `Watch`](#runtime-watch)
  - [Кнопки и жесты](#кнопки-и-жесты)
  - [Время](#время)
  - [display_map и letters (генерация данных)](#display_map-и-letters-генерация-данных)
- [Как написать свой фейс](#как-написать-свой-фейс)
- [Тесты](#тесты)
- [legacy/](#legacy)

---

## Плата

Прошивка предназначена для **платы замены Casio F-91W «Pluto»** (исходный
проект pluto-fw), собранной на микроконтроллере **MSP430FR6972**
(Texas Instruments, FRAM, ~64 КБ памяти). Часы сохраняют родное стекло F-91W —
жидкокристаллический дисплей с 10 семисегментными позициями и индикаторами.

Пин-аут платы (`crates/pluto-hw/src/main.rs`, по исходникам pluto-fw `target/hal`):

| Периферия  | Пин       | Примечание                          |
|------------|-----------|-------------------------------------|
| Кнопка Light | `PJ.0`  | вход с подтяжкой к земле, active-high |
| Кнопка Mode  | `PJ.2`  | вход с подтяжкой к земле, active-high |
| Кнопка Alarm | `P9.4`  | вход с подтяжкой к земле, active-high |
| Зуммер      | `P7.3`   | заглушка-«квадрат»; TODO: перевести на TA0 |
| Подсветка   | `P1.0`   | обычный GPIO; TODO: PWM через TA0  |
| Дисплей     | LCD_C    | 3-mux, charge pump, контраст 15    |

Часы тикают с интервалом **250 мс** (`TICK_MS`), так же работает и эмулятор.

> **Статус `pluto-hw` — WIP.** Крейт не входит в workspace и на этом
> компьютере ещё не собирался: нужен nightly Rust с целью `msp430-none-elf`
> и линкер TI `msp430-elf-gcc`. Драйвер RTC и зуммера, а также соответствие
> `display_map` реальной плате пока не проверены. Подробности — в
> [`crates/pluto-hw/README.md`](crates/pluto-hw/README.md).

---

## Быстрый старт: эмулятор

Для разработки фейсов достаточно эмулятора — он гоняет ту же самую логику
(собранную в WASM), что пойдёт на плату.

```sh
# 1. Цель WASM (один раз)
rustup target add wasm32-unknown-unknown

# 2. Сборка эмулятора (WASM + страница)
make -C emulator

# 3. Запуск
python3 -m http.server -d emulator/build
# открыть http://localhost:8000/watch.html
```

Управление:

- **кнопки на корпусе** часов на странице: Light / Mode / Alarm;
- **клавиатура**: `W` = Light, `S` = Mode, `D` = Alarm (или стрелки
  ↑ / ↓ / →);
- **удержание**: удерживать кнопку мышью — эмулятор сам шлёт повторы;
  на клавиатуре повторы зависят от OS key repeat.

---

## Сборка и прошивка реальной платы

Крейт `pluto-hw` собран автономно (не из корня workspace), т.к. требует
nightly MSP430-тулчейна.

```sh
# Установить тулчейн (см. crates/pluto-hw/rust-toolchain.toml):
#   nightly + rust-src + цель msp430-none-elf
# А также линкер TI: msp430-elf-gcc
rustup target add --toolchain nightly msp430-none-elf
rustup component add --toolchain nightly rust-src

cd crates/pluto-hw
cargo build --release

# Прошивка через mspdebug (пример для отладчика rf2500 / MSP-FET):
mspdebug rf2500 'prog target/msp430-none-elf/release/pluto-hw'
```

Сборка использует `-Zbuild-std=core`, скрипт линковки `link.x` и память из
`memory.x` (RAM 2 КБ @ `0x1C00`, ROM ~46.8 КБ @ `0x4400`).

**Важно перед прошивкой на реальную плату** (всё это TODO на текущий момент):

1. `display_map/display_map.json` — заполнить соответствие «стекло → LCD_C»
   по схеме платы и перегенерировать (`python3 tools/gen_display_map.py`).
2. Подключить **RTC_C** для настоящего времени (сейчас время считается от
   фиксированного момента загрузки).
3. Перевести **зуммер** с GPIO-заглушки на таймер TA0 (SMCLK).
4. Проверить распиновку LCD (`lcd.rs`) на реальной плате.

---

## Как устроен фреймворк

### Крейты

| Крейт           | Путь                     | Что это |
|-----------------|--------------------------|---------|
| `pluto-core`    | `crates/pluto-core`      | Фреймворк: `no_std`, без зависимостей. Traits, runtime, жесты, дисплей, время |
| `pluto-faces`   | `crates/pluto-faces`     | Набор фейсов (программ часов): `SimpleClock`, `Alarm`, `SimpleAlarm`, `Timer`; перечисление `Faces`. Какие из них попадают в сборку — задаётся в `faces.toml` |
| `pluto-emu`     | `crates/pluto-emu`       | Мост в WASM: `pluto_init` / `pluto_tick` / `pluto_button` + импорты `js_*` |
| `pluto-hw`      | `crates/pluto-hw`        | Прошивка MSP430: главный цикл + драйвер LCD_C (автономный крейт) |

`pluto-core` — фундамент. Его публичное API:

```rust
pub mod display;      // Display (стекло) + DigitDisplay (удобные отрисовщики)
pub mod display_map;  // генерация: display_map/display_map.rs
pub mod face;         // trait Face, FaceContext, жесты, аккорды, AlarmAction
pub mod font;         // FONT (позиции сегментов), DIGIT_SEGS, INDICATORS
pub mod hardware;     // Hardware: подсветка, зуммер, мелодии
pub mod input;        // ButtonScanner: распознавание жестов
pub mod letters;      // генерация: display_map/letters.rs
pub mod time;         // DateTime, Weekday, Month
pub mod watch;        // Watch<F>: runtime + FaceSet
```

Ключевая идея: **фейсы — это обычные структуры**, реализующие `trait Face`.
Они не знают ни о железе, ни о WASM; весь доступ к дисплею и эффектам идёт
через `Hardware` (суженную до конкретной платформы). Поэтому один и тот же
фейс одинаково работает в эмуляторе и на плате.

### Какие фейсы попадают в сборку

Файл `crates/pluto-faces/faces.toml` определяет, какие фейсы компилируются
в прошивку:

```toml
# crates/pluto-faces/faces.toml
faces = ["simple_clock", "simple_alarm", "timer"]
```

`simple_clock` обязателен всегда (это фейс по умолчанию, с которого часы
стартуют). Фейс, которого нет в списке, полностью выпадает из бинарника: его
модуль не компилируется, а цикл по Mode, сборка WASM и прошивка содержат
только перечисленные фейсы. Скрипт `build.rs` читает этот файл и превращает
каждый перечисленный фейс во флаг `face_*` (`face_simple_clock`,
`face_alarm`, `face_simple_alarm`, `face_timer`), которым в `lib.rs` закрыты
модули и варианты перечисления.

```sh
make -C emulator    # пересобрать эмулятор с новым набором фейсов
node tools/emu_test.mjs  # проверки идут только для фейсов из сборки
```

### Стекло F-91W и координаты сегментов

Все координаты — **стеклянные** `(com, seg)` — те же, что в SVG-скине
эмулятора и в таблице `FONT` (`crates/pluto-core/src/font.rs`). Каждый драйвер
(эмулятор, LCD_C) переводит их в свои биты через `display_map`.

Позиции цифр (индекс от 0):

```
  0  1       2  3        4  5 : 6  7 : 8  9
день недели  число      ЧЧ     ММ     СС
```

- **0–1** — буквы дня недели / метки режимов (SU, MO, TU, …, AL, ST).
  Рисуются через `set_char`, а не `set_digit`: набор сегментов на букву
  лежит в `letters/letters.json`.
- **2–3** — число месяца.
- **4–9** — ЧЧ:ММ:СС (без ведущего нуля у часов — фейс сам решает).
- **Индикаторы** — `Signal`, `Bell`, `Pm`, `H24`, `Lap`
  (`font::Indicator`, координаты в `INDICATORS`).

У стекла F-91W некоторые сегменты общие (например, день месяца не умеет
рисовать корректные «десятки») — в таблице `FONT` такие ячейки помечены
`(-1, -1)` и пропускаются.

### Runtime `Watch`

`Watch<F: FaceSet>` владеет **всеми** фейсами сразу (массив `F::Faces`).
Переключение кнопкой Mode лишь меняет активный фейс, а состояние каждого
сохраняется — поэтому будильник, настроенный в `Alarm`, не теряется при
переходе на `SimpleClock` и обратно.

Тиканье (`Watch::tick`, раз в 250 мс):

1. авто-выключение подсветки (~3 с после нажатия Light);
2. почасовой сигнал, если включён `chime`;
3. `background_tick()` для **каждого** фейса — фоновые действия, которые
   должны работать независимо от видимого фейса (например, срабатывание
   будильника). Фейс может вернуть `true` и попросить переключиться на себя;
4. `tick()` только активного фейса (он и рисует экран).

Фейсы рисуют весь экран целиком на каждом тике: записи сегментов
идемпотентны, поэтому перерисовка дешёвая и безопасная.

### Кнопки и жесты

Раскладка: **Light**, **Mode**, **Alarm**. Обработка в runtime:

- **Mode** — целиком занимается runtime: быстрый `Press` переключает фейсы.
- **Light** — `Hold` включает подсветку (с авто-выключением); все жесты
  также передаются активному фейсу.
- **Alarm** — первым делом получает фейс; если фейс не «съел» нажатие и это
  `Press`, runtime выполняет глобальное действие фейса `alarm_action()`
  (переключение 12/24-часового формата или почасового сигнала).

Жесты распознаёт `ButtonScanner` (`crates/pluto-core/src/input.rs`):

| Жест     | Условие                                        |
|----------|------------------------------------------------|
| `Press`  | быстрый тап — срабатывает **на отпускание**    |
| `Double` | два тапа в пределах 400 мс (`DOUBLE_CLICK_MS`) |
| `Hold`   | удержание > 750 мс (`HOLD_DELAY_MS`), автоповтор каждые 250 мс (`REPEAT_MS`); первый повтор также считается нажатием удержания |
| аккорд   | две кнопки нажаты одновременно → `ChordEvent` на отпускание обеих; press/hold кнопок аккорда подавляются |

Структуры жестов — в `pluto-core::face`:

```rust
GestureEvent { button: ButtonId, kind: GestureKind } // Light | Mode | Alarm
GestureKind  ::= Press | Hold | Double
ChordEvent   { first: ButtonId, second: ButtonId }
AlarmAction  ::= H24Toggle | ChimeToggle
```

### Время

`time::DateTime` строится из миллисекунд с эпохи Unix
(`DateTime::from_epoch_ms`) — алгоритмом Ховарда Хиннанта. Поля:

```rust
DateTime { secs: u64, ms: u16, year: u16, month: Month,
           day: u8, weekday: Weekday, hour: u8, minute: u8, second: u8 }
```

Время платформа подаёт сама: эмулятор — реальные часы браузера, плата —
RTC (пока заглушка). Фейсы время не задают, а только читают из `FaceContext`.

### display_map и letters (генерация данных)

- **`display_map/`** — соответствие «стеклянный сегмент → бит LCD».
  Единственный источник правды — `display_map.json`; Rust-таблица
  `display_map.rs` генерируется `tools/gen_display_map.py`. Сейчас это
  тождественное отображение (верно для эмулятора); владельцы реальных плат
  правят JSON под свою разводку.
- **`letters/`** — набор сегментов для букв дня недели.
  Правится в `letters.json` (визуально — `emulator/letters.html`),
  компилируется `tools/gen_letters.py` → `display_map/letters.rs`.

---

## Как написать свой фейс

Фейс — это структура + реализация `trait Face`. Пошагово:

### 1. Создайте модуль

`crates/pluto-faces/src/my_face.rs`:

```rust
use pluto_core::face::{ButtonId, Face, FaceContext, GestureEvent, GestureKind};
use pluto_core::{DigitDisplay, Hardware};

/// Мой фейс. Любое состояние, которое должно переживать переключение
/// Mode, храните прямо здесь (поле структуры).
#[derive(Clone, Copy, Default)]
pub struct MyFace {
    count: u8, // пример: собственное состояние
}
```

### 2. Реализуйте `Face`

```rust
impl Face for MyFace {
    // Вызывается один раз, когда фейс становится активным.
    fn init(&mut self, _ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.clear_all();
    }

    // Периодический тик активного фейса: здесь рисуем экран.
    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        hw.set_digit(4, ctx.time.hour / 10);
        hw.set_digit(5, ctx.time.hour % 10);
        hw.set_digit(6, ctx.time.minute / 10);
        hw.set_digit(7, ctx.time.minute % 10);
        // ...
    }

    // Фоновый тик: работает и когда фейс не активен.
    // Верните true, чтобы runtime переключился на этот фейс.
    fn background_tick(&mut self, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    // Обработка кнопки. Верните true, если жест полностью обработан —
    // тогда runtime НЕ выполнит глобальное действие кнопки Alarm.
    fn button(&mut self, event: GestureEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        match event {
            GestureEvent { button: ButtonId::Alarm, kind: GestureKind::Press } => {
                self.count = self.count.wrapping_add(1);
                true // съели нажатие
            }
            _ => false,
        }
    }

    // Обработка аккорда (две кнопки одновременно).
    fn chord(&mut self, event: pluto_core::ChordEvent, _ctx: &FaceContext, _hw: &mut impl Hardware) -> bool {
        false
    }

    // Глобальное действие кнопки Alarm, когда фейс не съел нажатие.
    fn alarm_action(&self) -> pluto_core::face::AlarmAction {
        pluto_core::face::AlarmAction::H24Toggle
    }
}
```

### 3. Зарегистрируйте фейс

В `crates/pluto-faces/src/lib.rs`:

1. `mod my_face;` и `pub use my_face::MyFace;`;
2. добавьте вариант в `enum Faces { SimpleClock(SimpleClock), Alarm(Alarm), MyFace(MyFace) }`;
3. добавьте делегирование во все методы `impl Face for Faces`;
4. добавьте экземпляр в `static ALL_FACES` (порядок = порядок цикла по Mode);
5. добавьте `"my_face"` в список `faces` в `crates/pluto-faces/faces.toml` и
   запись `("my_face", "face_my_face")` в таблицу `KNOWN` в
   `crates/pluto-faces/build.rs`.

### 4. Проверьте

```sh
cargo build                     # workspace (core + faces)
cargo test                      # юнит-тесты
make -C emulator && node tools/emu_test.mjs   # интеграционные проверки эмулятора
```

Полезные приёмы:

- **Отрисовка цифр** — `set_digit(pos, d)` / `clear_digit(pos)`
  (`DigitDisplay`). Не забывайте гасить неиспользуемые позиции.
- **Буквы/метки** — `set_char(pos, b'A')` (позиции 0–1).
- **Индикаторы** — `set_indicator(Indicator::Bell, true)`.
- **Зуммер** — `hw.beep()` (короткий), `hw.beep_ms(ms)`, `hw.melody(&notes)`,
  `hw.stop_melody()` (`Note { freq_hz, ms }`, до `MAX_MELODY_NOTES` нот).
- **Моргание** — ориентируйтесь на `ctx.time.ms`: интервал 250 мс удобно
  совпадает с тиком; см. пример моргания в `Alarm` (`Alarm::draw_edit`).
- **Долгое действие по удержанию** — `GestureKind::Hold` (автоповтор),
  `Double` для быстрого двойного тапа.

---

## Тесты

- `cargo test` — юнит-тесты `pluto-core` (жесты `ButtonScanner`, время,
  буквы в `tests/letters.rs`).
- `node tools/emu_test.mjs` — интеграционные проверки эмулятора: собирают
  WASM (`make -C emulator`), прогоняют сценарии нажатий/тиков и сверяют
  зажжённые сегменты, подсветку и зуммер. Проверки следуют `faces.toml`:
  тесты фейса идут только когда этот фейс есть в сборке.

---

## legacy/

Старое поколение фреймворка (модель «apps»: launcher/menu/settings, своя
версия `pluto-core`/`pluto-emu`) сохранено для справки. Актуальная разработка
идёт в `crates/`, на модели **фейсов**.
