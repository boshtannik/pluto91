# User manual

**Pluto** is a Casio F-91W-style wristwatch. It is controlled with three buttons:

| Button | Emulator (keyboard) |
|--------|---------------------|
| Light  | `W` (or ↑)          |
| Mode   | `S` (or ↓)          |
| Alarm  | `D` (or →)          |

In the emulator the buttons can also be clicked with the mouse on the watch
case shown on the page.

---

## Buttons and gestures

| Gesture       | What it is                                                |
|---------------|-----------------------------------------------------------|
| **Press**     | quick tap — fires **on release**                          |
| **Double**    | two taps within 0.4 s                                     |
| **Hold**      | hold > 0.75 s; while held, repeats fire automatically every 0.25 s |
| **Chord**     | two buttons pressed at the same time                      |

---

## Faces (modes)

The watch has three faces — **Time**, **Alarm** and **Simple Alarm**. A short
**Mode** press switches between them. Each face keeps its state across
switches: alarm settings survive leaving to Time and coming back.

> Which faces are present on a given build is set in
> `crates/pluto-faces/faces.toml` — a face not listed there is left out of the
> firmware and the emulator entirely (the default build has **Time** and
> **Simple Alarm**).

---

## Time face (SimpleClock)

Shows the time and the date:

```
 weekday     day         HH:MM:SS
 SU, MO, …   1..31       hour without a leading zero
```

Indicators:

| Indicator | Meaning                                   |
|-----------|-------------------------------------------|
| **H24**   | 24-hour format is on                      |
| **PM**    | 12-hour format, afternoon                 |
| **SIG**   | hourly chime is on                        |

Controls:

- **Alarm** — a short beep and a toggle of the **12/24-hour format**
  (the H24 indicator).
- **Light + hold** — backlight for ~3 seconds (turns off by itself).
- **Mode** — switch to the next face.

---

## Alarm face (Alarm)

One alarm per weekday (Sun…Sat). Each alarm has: weekday, hours, minutes and
a state (on/off).

### View mode

```
 AL     count       HH:MM
        of enabled  current time (a live clock)
        alarms
```

- The **AL** letters mean this is the alarm face.
- The top-right digit shows **how many alarms are enabled**.
- The **Bell** indicator is on if at least one alarm is enabled.

Controls in the view mode:

- **Alarm** — toggles the **hourly chime** (the SIG indicator).
- **Light** — enter the alarm settings.
- **Mode** — go to the next face.

### Settings

A short **Light** press in the view mode enters the settings. Fields advance
in the order: **day → hours → minutes → on/off**. The selected field
**blinks**.

While editing, the display shows which field is being edited:

| Letters    | Field              |
|------------|--------------------|
| weekdays   | alarm day (Sun…Sat)|
| `HO`       | hours              |
| `MI`       | minutes            |
| `AC`       | on/off             |

The on/off state is shown as **ON / OF** in the seconds digits.

| Action            | Result                                           |
|-------------------|--------------------------------------------------|
| **Light**         | go to the next field; on the on/off field — exit the settings |
| **Light + hold**  | exit the settings (at any point)                 |
| **Alarm**         | increase the value by 1                          |
| **Alarm + double**| increase the value by 5                          |
| **Alarm + hold**  | reset the value to 0 (day — Sun, hours/minutes — 00) |
| **Alarm + Light** | set the alarm to the **current time**            |

Nice touches:

- On entering the settings a **never-configured** alarm (disabled, 00:00) is
  automatically filled with the **current time** — handy when you want it to
  go off "in a few minutes".
- While scrolling fast (hold), the value is shown steadily instead of
  blinking so it stays readable; blinking resumes right after you stop.

### Firing

When an enabled alarm's time arrives, the watch **switches to the Alarm face
by itself** (no matter what you were looking at) and rings. The ring lasts up
to **2 minutes** or until you press any button. Switching the face with Mode
also stops the ring.

While the alarm rings, the face shows the **current time in full** (with
seconds) and the **Bell indicator blinks** every half second.

---

## Simple Alarm face (SimpleAlarm)

A classic Casio-style single alarm — no weekdays, just one time of day.

### View mode

```
 AL            HH:MM
               alarm time
```

- The **AL** letters mean this is the alarm face.
- The alarm time is shown in the current 12/24-hour format, without a leading
  zero (seconds are not shown).
- The **Bell** indicator is on when the alarm is enabled.

Controls in the view mode:

- **Alarm** — turn the alarm on/off (the Bell indicator).
- **Light** — enter the settings.
- **Mode** — go to the next face.

### Settings

A short **Light** press in the view mode enters the settings. Fields advance
in the order: **hours → minutes**. The selected field **blinks**; the display
shows which field is being edited — `HO` (hours) or `MI` (minutes).

| Action            | Result                                           |
|-------------------|--------------------------------------------------|
| **Light**         | go to the next field; on minutes — exit the settings |
| **Light + hold**  | exit the settings (at any point)                 |
| **Alarm**         | increase the value by 1                          |
| **Alarm + double**| increase the value by 5                          |
| **Alarm + hold**  | reset the value to 0                             |
| **Alarm + Light** | set the alarm to the **current time**            |

Nice touches:

- On entering the settings a **never-configured** alarm (disabled, 00:00) is
  automatically filled with the **current time** — handy when you want it to
  go off "in a few minutes".
- The **Alarm + Light** chord re-seeds the alarm with the current time from
  anywhere; in the view mode it also opens the settings so you can nudge the
  time right away.
- While scrolling fast (hold), the value is shown steadily instead of
  blinking so it stays readable; blinking resumes right after you stop.

### Firing

Same as the alarm face: when the time arrives the watch **switches to this
face by itself** and rings up to **2 minutes** or until you press any button.
The ring starts exactly at the top of the minute (HH:MM:00), so a freshly
set alarm never rings in the middle of a minute.

While the alarm rings, the face shows the **current time in full** (with
seconds) and the **Bell indicator blinks** every half second; once the ring
ends it goes back to the alarm time.

---

## Hourly chime

At the top of every hour the watch beeps briefly if the hourly chime is on
(the **SIG** indicator). It is toggled with the **Alarm** button on the Alarm
face (in the view mode); the SIG indicator is also visible on the Time face.
