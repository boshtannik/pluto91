import { readFile } from 'node:fs/promises';
const buf = await readFile('target/wasm32-unknown-unknown/release/pluto_emu.wasm');
let segs = new Map(); // "com,seg" -> bool, current LCD state
let backlights = [];
let beeps = []; // [freq, ms, delay]
let t = 1_700_000_000_000; // ms since epoch
const imports = { env: {
  js_clear: () => { segs.clear(); },
  js_seg: (com, seg, on) => { segs.set(`${com},${seg}`, on === 1); },
  js_backlight: (on) => { backlights.push(on); },
  js_beep: (freq, ms, delay) => { beeps.push([freq, ms, delay]); },
  js_stop_melody: () => {},
  js_now: () => t,
  js_panic: () => { console.log('panic'); },
}};
const { instance } = await WebAssembly.instantiate(buf, imports);
const ex = instance.exports;
ex.pluto_init();
const onNow = () => new Set([...segs].filter(([,v]) => v).map(([k]) => k));
let fail = 0;
const check = (k, v) => { if (!v) fail++; console.log(v ? 'ok  ' : 'FAIL', k); };

// One press = down then up, with a large enough time gap between presses so
// the gesture scanner never mistakes them for a double-click.
const press = (id) => {
  t += 2000;
  ex.pluto_button(id, 1);
  t += 2000;
  ex.pluto_button(id, 0);
};
// Light: one press then one hold auto-repeat (turns the backlight on).
const holdLight = () => {
  t += 2000;
  ex.pluto_button(0, 1); // Press
  t += 2000;
  ex.pluto_button(0, 1); // Hold (>= HOLD_DELAY_MS)
  t += 2000;
  ex.pluto_button(0, 0);
};
// Two buttons pressed together: `a` goes down, `b` follows within the chord
// window, then both are released. The pair is delivered to the active face as
// a chord instead of two separate presses.
const chordPress = (a, b) => {
  t += 2000;
  ex.pluto_button(a, 1); // first down (press held undelivered)
  t += 50;
  ex.pluto_button(b, 1); // second down within the window -> chord
  t += 2000;
  ex.pluto_button(b, 0); // release second
  t += 2000;
  ex.pluto_button(a, 0); // release first
};

// --- SimpleClock: 2024-02-29 12:34:56 UTC (Thursday, weekday=4) ---
ex.pluto_tick(1_709_210_096_000);
let on = onNow();
check('hour ones=2 -> A(2,20) D(0,21) G(1,20)', on.has('2,20') && on.has('0,21') && on.has('1,20'));
check('day=29 -> pos2 D(1,9) pos3 A(0,7)', on.has('1,9') && on.has('0,7'));
check('weekday TH -> T=A(0,13)+ninth(1,14); H=B(1,11)E/F(1,12)G(2,12); A(0,11)/D(2,11) off',
  on.has('0,13') && on.has('1,14') && on.has('1,11') && on.has('1,12') && on.has('2,12')
  && !on.has('0,11') && !on.has('2,11'));
check('24H on, Signal off (chime disabled by default)',
  on.has('2,16') && !on.has('0,17'));

// --- leading-zero suppression (still SimpleClock): Fri 2026-08-07 06:40:00 ---
ex.pluto_tick(Date.UTC(2026, 7, 7, 6, 40, 0));
on = onNow();
check('weekday FR -> F=top(0,13)+LL(2,14)+leg(1,15)+UL(0,14); R=diag(2,14)(1,15)+ninth(0,12)',
  on.has('0,13') && on.has('2,14') && on.has('1,15') && on.has('0,14') && !on.has('1,13') && !on.has('2,13')
  && on.has('0,11') && on.has('1,11') && on.has('1,12') && on.has('2,12') && on.has('0,12'));
const blank = (pos) => {
  const off = [[1,18],[2,19],[0,19],[0,18],[2,18],[1,19],[1,9],[0,9],[2,9],[0,10]];
  return off.every(s => !on.has(`${s[0]},${s[1]}`));
};
check('leading zeros blanked (day=7, hour=6)', blank());
check('day ones=7 shown', on.has('0,7'));

// --- hardware effects on SimpleClock ---
press(0); // Light quick press -> NO backlight (saves battery)
check('light press -> no backlight', backlights.at(-1) !== 1);
holdLight(); // Light held -> backlight on
check('light hold -> backlight on', backlights.at(-1) === 1);
ex.pluto_tick(Date.UTC(2026, 7, 7, 6, 40, 5)); // 3+ s later -> auto-off
check('backlight auto-off after 3s', backlights.at(-1) === 0);
const base = beeps.length;
press(2); // Alarm -> single beep
check('alarm->beep', beeps.length === base + 1 && beeps[base][0] === 2400 && beeps[base][1] === 60);
press(2); // Alarm again: back to 24h (Alarm also toggles the format)
const n0 = beeps.length;
ex.pluto_tick(Date.UTC(2026, 7, 7, 6, 59, 59)); // same hour, no chime
ex.pluto_tick(Date.UTC(2026, 7, 7, 7, 0, 0));   // hour change, chime disabled
check('no hourly chime when disabled', beeps.length === n0);

// --- Alarm toggles 12/24h format. t is now ~22:13:38, so in 24h the hour
// tens are "2", in 12h they become "1" and the PM indicator lights up. ---
ex.pluto_tick(t);
on = onNow();
check('24h before toggle: H24 on, hour tens=2 (E(0,18))',
  on.has('2,16') && !on.has('2,17') && on.has('0,18'));
press(2); // Alarm press -> 12h; next tick redraws
ex.pluto_tick(t);
on = onNow();
check('12h after toggle: H24 off, PM on, hour tens=1 (E(0,18) off)',
  !on.has('2,16') && on.has('2,17') && !on.has('0,18'));
press(2); // Alarm press -> back to 24h
ex.pluto_tick(t);
on = onNow();
check('24h again: H24 on, PM off',
  on.has('2,16') && !on.has('2,17'));

// --- Mode cycles between the clock and the alarm face ---
press(1); // Mode -> Alarm face
ex.pluto_tick(Date.UTC(2026, 7, 7, 6, 40, 30));
on = onNow();
check('mode -> alarm view: AL top, count 00, live clock, Bell off, H24 on',
  on.has('0,13') && on.has('2,13') && on.has('1,15')   // A (pos0)
  && on.has('2,11') && on.has('1,12')                  // L (pos1)
  && on.has('0,7') && on.has('1,7') && on.has('2,7')   // count ones 0 (pos3)
  && on.has('2,6') && on.has('2,8') && on.has('0,8')   // count ones 0 (pos3, rest)
  && !on.has('0,9') && !on.has('2,9') && !on.has('0,10') // count tens blank
  && on.has('2,23') && on.has('0,0')                   // live minutes 40
  && on.has('0,4') && on.has('2,4')                    // live seconds 30
  && on.has('2,16')                                    // H24 on (global 24h)
  && !on.has('2,17') && !on.has('0,17') && !on.has('0,16')); // PM/Signal/Bell off
press(1); // Mode -> back to SimpleClock
ex.pluto_tick(t);
on = onNow();
check('mode back -> clock face drawn',
  on.has('0,13') && on.has('1,14') && on.has('2,20'));

// --- Alarm editing: set Monday 07:05 ON ---
const BLINK_ON  = 1_700_000_000_000; // ms in [0,250)   -> blinking field visible
const BLINK_OFF = 1_700_000_000_250; // ms in [250,500) -> blinking field hidden
const pressAlarm = () => {
  t += 2000; ex.pluto_button(2, 1);
  t += 2000; ex.pluto_button(2, 0);
};
const pressLight = () => {
  t += 2000; ex.pluto_button(0, 1);
  t += 2000; ex.pluto_button(0, 0);
};
// Alarm: a fast double press (second click within 400ms -> Double gesture).
const doublePress = () => {
  t += 2000; ex.pluto_button(2, 1); // Press (first click)
  t += 100;  ex.pluto_button(2, 0); // release
  t += 100;  ex.pluto_button(2, 1); // within 400ms -> Double
  t += 2000; ex.pluto_button(2, 0);
};
// Alarm: press then hold long enough for the auto-repeat to fire once.
const holdAlarm = () => {
  t += 2000; ex.pluto_button(2, 1); // Press -> +1
  t += 2000; ex.pluto_button(2, 1); // Hold auto-repeat -> +1
  t += 2000; ex.pluto_button(2, 0);
};

press(1); // -> Alarm face again
ex.pluto_tick(t);
// Sunday 00:00: the unconfigured Sunday alarm gets seeded with this time,
// i.e. stays 00:00, so the edit flow below starts from all zeros.
t = Date.UTC(2026, 7, 9, 0, 0, 0);
pressLight(); // Light press enters the edit mode (no hold)
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: day field shows SU (blink visible)',
  on.has('2,15') && on.has('1,15') && on.has('1,11') && on.has('1,12'));
ex.pluto_tick(BLINK_OFF);
on = onNow();
check('edit: day field hidden after 250ms (blink off)',
  !on.has('2,15') && !on.has('1,15') && !on.has('1,11') && !on.has('1,12'));

pressAlarm(); // Sunday -> Monday
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: day -> MO after Alarm press',
  on.has('1,13') && on.has('2,14') && on.has('0,11') && on.has('1,11'));

pressLight(); // -> Hour
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: hour field, label HO, hours 00',
  on.has('1,13') && on.has('2,13') && on.has('1,15')   // H (pos0)
  && on.has('0,11') && on.has('1,11')                  // O (pos1)
  && on.has('2,20') && !on.has('1,19'));               // hours "00"
// A double press adds five units in total (first click +1, second +4).
doublePress(); // hour 00 -> 05
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: double press adds 5 (hour 05)',
  on.has('1,20') && !on.has('2,21') && !on.has('0,20'));
ex.pluto_tick(BLINK_OFF);
on = onNow();
check('edit: focus steady after double (05 visible on blink-off phase)',
  on.has('1,20') && !on.has('2,21'));
doublePress(); // hour 05 -> 10
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: second double adds 5 (hour 10)',
  on.has('2,21') && on.has('0,20') && on.has('1,17') && !on.has('1,18'));
// Holding the button resets the focused value to zero.
holdAlarm(); // Press +1 (11), Hold -> reset 00
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: hold resets hour to 00',
  on.has('2,21') && on.has('0,20') && on.has('1,18') && !on.has('1,20'));
ex.pluto_tick(BLINK_OFF);
on = onNow();
check('edit: focus steady after hold reset (00 visible on blink-off phase)',
  on.has('2,21') && on.has('0,20') && on.has('1,18'));
for (let i = 0; i < 7; i++) pressAlarm(); // hour 00 -> 07
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: hour 07 shown', on.has('2,20') && on.has('1,21') && !on.has('1,20'));

pressLight(); // -> Minute
for (let i = 0; i < 5; i++) pressAlarm(); // minute 00 -> 05
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: minute 05 shown',
  on.has('0,22') && on.has('2,1') && on.has('0,1') && on.has('1,1'));

pressLight(); // -> Status
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: status OF shown (seconds), label AC',
  on.has('0,13') && on.has('2,11')           // A(pos0) C(pos1)
  && on.has('0,4') && !on.has('1,3')         // seconds pos8 = O
  && on.has('2,4') && on.has('1,4') && !on.has('1,6')); // seconds pos9 = F
pressAlarm(); // OF -> ON
ex.pluto_tick(BLINK_ON);
on = onNow();
check('edit: status ON shown (seconds)',
  on.has('0,4') && on.has('1,6') && on.has('0,5') && on.has('1,5')
  && !on.has('2,4') && !on.has('1,4'));

// Light press on the status field exits the edit mode (1 alarm active, Bell on)
pressLight();
ex.pluto_tick(BLINK_ON);
on = onNow();
check('view after edit: count 01, Bell on',
  on.has('1,7') && on.has('2,7')                // count ones 1 (pos3)
  && !on.has('0,7')                             // ones 0 off
  && !on.has('0,9') && !on.has('2,9') && !on.has('0,10') // count tens blank
  && on.has('0,16'));                           // Bell

// --- alarm fires at Mon 07:05 (2026-08-10 is a Monday) ---
const nFired = beeps.length;
ex.pluto_tick(Date.UTC(2026, 7, 10, 7, 5, 0));
check('alarm beeps when time matches', beeps.length === nFired + 1);
ex.pluto_tick(Date.UTC(2026, 7, 10, 7, 5, 30)); // still within the ring window
check('ringing continues within the minute', beeps.length === nFired + 2);
ex.pluto_tick(Date.UTC(2026, 7, 10, 7, 7, 0)); // exactly 2 minutes after firing
check('ringing auto-stops after 2 minutes', beeps.length === nFired + 2);
ex.pluto_tick(Date.UTC(2026, 7, 11, 7, 5, 0)); // Tue has no alarm enabled
check('disabled weekday does not fire', beeps.length === nFired + 2);
ex.pluto_tick(Date.UTC(2026, 7, 17, 7, 5, 0)); // next Monday again -> fires
const nRing = beeps.length;
check('alarm re-fires on a later week', beeps.length === nFired + 3);
ex.pluto_tick(Date.UTC(2026, 7, 17, 7, 5, 30)); // still ringing
press(2); // any button silences the ringing
ex.pluto_tick(Date.UTC(2026, 7, 17, 7, 5, 31));
check('any button stops the ringing', beeps.length === nRing + 1);
ex.pluto_tick(Date.UTC(2026, 7, 17, 7, 5, 32));
check('ringing does not resume after button stop', beeps.length === nRing + 1);

// --- Mode exit from the middle of an edit resets the face to view ---
// Monday is currently seeded to 09:37 (by the chord above); set it back to
// 07:05 to keep the configured alarm intact for the auto-switch test, then
// jump past the firing minute so the ticks below don't ring early.
t = Date.UTC(2026, 7, 17, 7, 5, 0);
pressLight(); // enter edit mode (seeds Monday 07:05)
t = Date.UTC(2026, 7, 17, 8, 0, 0);
ex.pluto_tick(BLINK_ON); // day field blinks now
press(1); // Mode -> SimpleClock (leaves the edit mid-way)
ex.pluto_tick(t);
press(1); // Mode -> Alarm again
ex.pluto_tick(t);
on = onNow();
check('alarm face returns to view after Mode (AL top)',
  on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12'));

// --- auto-switch: the watch jumps to the ringing face ---
press(1); // Mode -> SimpleClock
ex.pluto_tick(t);
on = onNow();
check('on SimpleClock before auto-switch (Bell off)',
  !on.has('0,16'));
const nAuto = beeps.length;
ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 0)); // next Monday, alarm fires
on = onNow();
check('auto-switch to Alarm when it fires',
  on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12') // AL (pos0-1)
  && on.has('1,7') && on.has('2,7') && !on.has('0,7')                 // count 01
  && on.has('0,16'));                                                 // Bell
check('auto-switch rings', beeps.length === nAuto + 1);
ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 1)); // still ringing
check('auto-switched ring continues', beeps.length === nAuto + 2);
press(2); // Alarm button (no-op in view) silences it
ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 2));
check('auto-switched ring stops on button', beeps.length === nAuto + 2);
ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 3));
check('no resume after stop (and still on Alarm view)',
  beeps.length === nAuto + 2 && onNow().has('0,13') && onNow().has('2,11'));

// --- editing: never-configured alarms start from "now"; configured ones keep ---
// --- their time; Alarm + Light together re-seed with the current time ---
// Monday is configured (07:05, enabled), so re-entering the edit mode must
// NOT clobber it.
t = Date.UTC(2026, 7, 31, 9, 37, 0);
ex.pluto_tick(t); // redraw Alarm view at 09:37
pressLight();     // enter edit: day is still Monday (configured) -> keeps 07:05
ex.pluto_tick(BLINK_ON);
on = onNow();
check('configured alarm keeps its time on re-enter (07:05)',
  on.has('0,19') && on.has('1,21')          // hours 07 (tens 0, ones 7)
  && on.has('2,22') && on.has('1,1')        // minutes 05 (tens 0, ones 5)
  && !on.has('1,17') && !on.has('1,23'));   // not 09 / 3x
// Scrolling to a day that was never configured (00:00, disabled) seeds it
// with the current time as the base value.
for (let i = 0; i < 6; i++) pressAlarm(); // Mon->Tue->...->Sun (unconfigured)
ex.pluto_tick(BLINK_ON);
on = onNow();
check('new alarm seeds with current time (09:37)',
  on.has('0,19') && on.has('1,17')          // hours 09 (tens 0, ones 9)
  && on.has('1,23') && on.has('2,10')       // minutes 37 (tens 3, ones 7)
  && !on.has('2,22') && !on.has('1,1'));    // not 0x / x5
// Back on Monday the configured alarm still has its saved time.
pressAlarm(); // Sun -> Mon
ex.pluto_tick(BLINK_ON);
on = onNow();
check('configured alarm keeps its time after scrolling (07:05)',
  on.has('0,19') && on.has('1,21') && on.has('2,22') && on.has('1,1')
  && !on.has('1,17') && !on.has('1,23'));
// Alarm + Light pressed together re-seeds with the current time, even for
// the previously configured Monday alarm.
chordPress(2, 0); // hold Alarm, then Light (a chord)
ex.pluto_tick(BLINK_ON);
on = onNow();
check('chord Alarm+Light re-seeds with current time (09:37)',
  on.has('0,19') && on.has('1,17')          // hours 09 (tens 0, ones 9)
  && on.has('1,23') && on.has('2,10')       // minutes 37 (tens 3, ones 7)
  && !on.has('2,22') && !on.has('1,1'));    // not 0x / x5
// A chord must not make the next tap of one of its buttons look like a
// double-click (a ghost +4 instead of a +1). We are in the Hour field on
// Monday 09:37; one plain Alarm press must step the hour to 10.
pressLight(); // Day -> Hour
ex.pluto_tick(BLINK_ON);
pressAlarm(); // plain press -> +1
ex.pluto_tick(BLINK_ON);
on = onNow();
check('chord does not ghost a double press (+1, hour 10)',
  on.has('2,19') && on.has('0,19')          // tens 1 (pos4)
  && on.has('2,20') && on.has('1,17')       // ones 0 (pos5)
  && !on.has('1,20'));                      // not a 3 (would be +4)
// Holding Light exits the edit back to the view.
holdLight();
ex.pluto_tick(t);
on = onNow();
check('hold Light exits edit to view (AL + live clock)',
  on.has('0,13') && on.has('2,13') && on.has('1,15') // AL
  && on.has('2,11') && on.has('1,12')
  && on.has('1,17') && on.has('1,23') && on.has('2,10')); // live 09:37

// --- Alarm button in the view mode toggles the hourly chime (SIG) ---
pressAlarm(); // view: Alarm button toggles the chime on
ex.pluto_tick(t);
on = onNow();
check('Alarm button in view toggles SIG on',
  on.has('0,17') && on.has('0,13') && on.has('2,11')); // SIG + AL
pressAlarm(); // ... and back off
ex.pluto_tick(t);
on = onNow();
check('Alarm button in view toggles SIG off', !on.has('0,17'));
// Leave the chime on and check it beeps at the top of the hour.
pressAlarm();
ex.pluto_tick(t);
press(1); // Mode -> SimpleClock
const nChime = beeps.length;
ex.pluto_tick(Date.UTC(2026, 8, 1, 12, 0, 0)); // Tue, no alarm set
check('hourly chime beeps at the top of the hour (SIG on)',
  beeps.length === nChime + 1 && onNow().has('0,17'));

// --- hold reacts: the first auto-repeat of a held button acts as its press ---
// Holding Mode must cycle to the alarm face while the button is still held
// (no wait for the release). We are on SimpleClock with the chime on.
t = Date.UTC(2026, 8, 1, 12, 0, 0);
ex.pluto_tick(t);
t += 2000;
ex.pluto_button(1, 1); // Mode down (fresh press, undelivered)
t += 2000;
ex.pluto_button(1, 1); // first Hold auto-repeat -> the hold's press -> cycle
t += 2000;
ex.pluto_button(1, 0); // release (held: no second cycle)
ex.pluto_tick(t);
on = onNow();
check('hold Mode cycles to Alarm view (AL + SIG)',
  on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12')
  && on.has('0,17'));

// --- holding Light turns the backlight on and still forms chords ---
// We are now on the Alarm view (chime on). Holding Light fires its press
// (view -> edit -> back out via the hold) and the backlight; pressing Alarm
// while Light is held must still be recognised as a chord.
ex.pluto_button(0, 1); // Light down
t += 2000;
ex.pluto_button(0, 1); // first Hold auto-repeat -> backlight on (+ press)
check('hold Light turns the backlight on', backlights.at(-1) === 1);
t += 2000;
ex.pluto_button(2, 1); // Alarm down while Light is held -> chord (Light, Alarm)
t += 2000;
ex.pluto_button(2, 0); // release Alarm
t += 2000;
ex.pluto_button(0, 0); // release Light -> chord delivered (no-op in view)
ex.pluto_tick(t);
on = onNow();
check('chord after a Light hold is still delivered (AL view unchanged)',
  on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12'));

// --- during a chord the held button's auto-repeat must not fire ---
// Enter the Alarm edit (Monday keeps its saved time), then hold Alarm and
// press Light. Alarm's hold auto-repeat fires while both buttons are down;
// the hybrid must suppress it (no day reset, no +1) so the chord alone
// re-seeds the selected day. If the hold leaked through, reset() would move
// the day to Sunday and the re-seed would land there.
t = Date.UTC(2026, 8, 1, 9, 30, 0);
ex.pluto_tick(t);
pressLight();     // Light press -> edit mode (Day field)
ex.pluto_tick(BLINK_ON);
t += 2000;
ex.pluto_button(2, 1); // Alarm down
t += 50;
ex.pluto_button(0, 1); // Light down -> chord (Alarm, Light)
t += 2000;             // Alarm's hold auto-repeat fires now; must be suppressed
t += 2000;
ex.pluto_button(0, 0); // release Light
t += 2000;
ex.pluto_button(2, 0); // release Alarm -> chord re-seeds Monday 09:30
ex.pluto_tick(BLINK_ON);
on = onNow();
check('chord suppresses the held button repeat (Monday, 09:30)',
  on.has('1,14')                // M of MO (not reset to Sunday SU)
  && !on.has('2,15')            // S top segment off
  && on.has('0,19') && on.has('1,17') && !on.has('0,20')  // hours 09 (E of 9 off)
  && on.has('1,23') && !on.has('2,22')                    // minutes tens 3 (F off)
  && on.has('2,10') && !on.has('1,1'));                   // minutes ones 0 (G off)

if (fail) process.exit(1);
console.log('ALL OK');
