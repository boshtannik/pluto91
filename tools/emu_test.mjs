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

// Which faces this build contains: read faces.toml (the same file build.rs
// turns into cfg flags), then verify the wasm matches.
const facesText = await readFile('crates/pluto-faces/faces.toml', 'utf8');
const enabled = new Set();
for (const raw of facesText.split('\n')) {
  const line = raw.split('#')[0].trim();
  if (!line.startsWith('faces')) continue;
  const m = line.match(/faces\s*=\s*\[(.*)\]/);
  if (!m) continue;
  for (const item of m[1].split(',')) {
    const name = item.trim().replace(/^"|"$/g, '').trim();
    if (name) enabled.add(name);
  }
}
if (!enabled.has('simple_clock')) throw new Error('faces.toml must always enable simple_clock');
// Faces in FaceSet order: simple_clock first, then the config order.
const ORDER = ['simple_clock', 'alarm', 'simple_alarm', 'timer'].filter((f) => enabled.has(f));
const idx = (name) => ORDER.indexOf(name);
if (ex.pluto_face_count() !== ORDER.length) {
  throw new Error(
    `wasm has ${ex.pluto_face_count()} faces but faces.toml lists ${ORDER.length} (${ORDER}) — rebuild: make -C emulator`);
}

const onNow = () => new Set([...segs].filter(([,v]) => v).map(([k]) => k));
let fail = 0;
const check = (k, v) => { if (!v) fail++; console.log(v ? 'ok  ' : 'FAIL', k); };

// 7-segment decoding helpers (mirror font.rs FONT / DIGIT_SEGS) so the timer
// tests can read whole digits off the segment map instead of listing segments.
const FONT = [
  [[0,13],[1,13],[2,13],[2,15],[2,14],[0,14],[1,15]],
  [[0,11],[1,11],[1,11],[2,11],[1,12],[1,12],[2,12]],
  [[1,9],[0,9],[2,9],[1,9],[0,10],[-1,-1],[1,9]],
  [[0,7],[1,7],[2,7],[2,6],[2,8],[0,8],[1,8]],
  [[1,18],[2,19],[0,19],[1,18],[0,18],[2,18],[1,19]],
  [[2,20],[2,21],[1,21],[0,21],[0,20],[1,17],[1,20]],
  [[0,22],[2,23],[0,23],[0,22],[1,22],[2,22],[1,23]],
  [[2,1],[2,10],[0,1],[0,0],[1,0],[2,0],[1,1]],
  [[2,2],[2,3],[0,4],[0,3],[0,2],[1,2],[1,3]],
  [[2,4],[2,5],[1,6],[0,6],[0,5],[1,4],[1,5]],
];
const DIGIT_SEGS = [0x3f,0x06,0x5b,0x4f,0x66,0x6d,0x7d,0x07,0x7f,0x6f];
// The digit currently drawn at a position, or -1 when it is blank.
const digitAt = (pos) => {
  for (let d = 0; d <= 9; d++) {
    let ok = true;
    for (let i = 0; i < 7; i++) {
      const [com, seg] = FONT[pos][i];
      if (com < 0) continue;
      if (segs.get(`${com},${seg}`) !== (((DIGIT_SEGS[d] >> i) & 1) === 1)) { ok = false; break; }
    }
    if (ok) return d;
  }
  return -1;
};

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

// Mode presses cycle the active face; keep a mirror of the active index so
// `goTo` can step the right number of presses regardless of the build's
// face count. (An alarm firing auto-switches the face: sync `faceIdx` there.)
let faceIdx = 0;
const mode = () => { press(1); faceIdx = (faceIdx + 1) % ORDER.length; };
const goTo = (name) => {
  for (let i = 0; i < ORDER.length; i++) {
    if (ORDER[faceIdx] === name) return;
    mode();
  }
  throw new Error(`goTo(${name}) failed: not in this build (${ORDER})`);
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

if (ORDER.includes('alarm')) {
  // --- Mode cycles between the clock and the alarm faces ---
  goTo('alarm'); // Mode -> Alarm face
  ex.pluto_tick(Date.UTC(2026, 7, 7, 6, 40, 30));
  on = onNow();
  check('mode -> alarm view: AL top, count 00, live clock, Bell off, H24 on',
    on.has('0,13') && on.has('2,13') && on.has('1,15')   // A (pos0)
    && on.has('2,11') && on.has('1,12')                  // L (pos1)
    && on.has('0,7') && on.has('1,7') && on.has('2,7')   // count ones 0 (pos3)
    && on.has('2,6') && on.has('2,8') && on.has('0,8')   // count ones 0 (pos3, rest)
    && !on.has('0,9') && !on.has('2,9') && !on.has('0,10') // count tens blank
    && on.has('2,23') && on.has('0,0')                   // live minutes 40
    && !on.has('0,4') && !on.has('2,4')                  // seconds hidden
    && on.has('2,16')                                    // H24 on (global 24h)
    && !on.has('2,17') && !on.has('0,17') && !on.has('0,16')); // PM/Signal/Bell off
  goTo('simple_clock'); // Mode x2 -> back to SimpleClock
  ex.pluto_tick(t);
  on = onNow();
  check('mode back -> clock face drawn',
    on.has('0,13') && on.has('1,14') && on.has('2,20'));
}

if (ORDER.includes('alarm')) {
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

  goTo('alarm'); // -> Alarm face again
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
  const nextFace = ORDER[(idx('alarm') + 1) % ORDER.length]; // SimpleAlarm, or SimpleClock if it is not built
  t = Date.UTC(2026, 7, 17, 7, 5, 0);
  pressLight(); // enter edit mode (seeds Monday 07:05)
  t = Date.UTC(2026, 7, 17, 8, 0, 0);
  ex.pluto_tick(BLINK_ON); // day field blinks now
  goTo(nextFace); // Mode -> next face (leaves the edit mid-way)
  ex.pluto_tick(t);
  goTo('alarm'); // Mode x2 -> back to Alarm
  ex.pluto_tick(t);
  on = onNow();
  check('alarm face returns to view after Mode (AL top)',
    on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12'));

  // --- auto-switch: the watch jumps to the ringing face ---
  goTo('simple_clock'); // Mode x2 -> SimpleClock
  ex.pluto_tick(t);
  on = onNow();
  check('on SimpleClock before auto-switch (Bell off)',
    !on.has('0,16'));
  const nAuto = beeps.length;
  ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 0)); // next Monday, alarm fires
  faceIdx = idx('alarm'); // the watch auto-switched to the Alarm face
  on = onNow();
  check('auto-switch to Alarm when it fires',
    on.has('0,13') && on.has('1,15') && on.has('2,11') && on.has('1,12') // AL (pos0-1)
    && on.has('1,7') && on.has('2,7') && !on.has('0,7')                 // count 01
    && on.has('0,16')                                                   // Bell (blink on)
    && on.has('0,4') && on.has('2,4'));                                 // ring: seconds 00
  check('auto-switch rings', beeps.length === nAuto + 1);
  ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 1)); // still ringing
  check('auto-switched ring continues', beeps.length === nAuto + 2);
  ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 1, 600)); // still ringing, ms=600
  check('Alarm Bell blinks off at 500ms into the second',
    !onNow().has('0,16') && beeps.length === nAuto + 3);
  press(2); // Alarm button (no-op in view) silences it
  ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 2));
  check('auto-switched ring stops on button', beeps.length === nAuto + 3);
  ex.pluto_tick(Date.UTC(2026, 7, 31, 7, 5, 3));
  check('no resume after stop (and still on Alarm view)',
    beeps.length === nAuto + 3 && onNow().has('0,13') && onNow().has('2,11'));

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
  goTo('simple_clock'); // Mode x2 -> SimpleClock
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
  faceIdx = (faceIdx + 1) % ORDER.length; // cycled to the Alarm face
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
}

if (ORDER.includes('simple_alarm')) {
  // --- SimpleAlarm: the Casio-style single alarm (no weekdays) ---
  goTo('simple_alarm'); // Mode -> SimpleAlarm
  t = Date.UTC(2026, 8, 1, 12, 0, 0);
  ex.pluto_tick(t);
  on = onNow();
  check('SimpleAlarm view: AL, alarm 00:00, Bell off',
    on.has('0,13') && on.has('2,13') && on.has('1,15')   // A (pos0)
    && on.has('2,11') && on.has('1,12')                  // L (pos1)
    && !on.has('2,19') && !on.has('0,19')                // hour tens blank (leading zero)
    && on.has('2,20') && on.has('2,21') && !on.has('1,20') // hour ones 0
    && on.has('0,22') && on.has('2,22') && !on.has('1,23') // minute tens 0
    && on.has('2,1') && on.has('2,10') && !on.has('1,1')   // minute ones 0
    && !on.has('0,4') && !on.has('2,4')                  // seconds hidden
    && !on.has('0,16') && on.has('2,16'));               // Bell off, H24 on

  const BLINK_ON  = 1_700_000_000_000; // ms in [0,250)   -> blinking field visible
  const pressAlarm = () => {
    t += 2000; ex.pluto_button(2, 1);
    t += 2000; ex.pluto_button(2, 0);
  };
  const pressLight = () => {
    t += 2000; ex.pluto_button(0, 1);
    t += 2000; ex.pluto_button(0, 0);
  };

  pressAlarm(); // view: Alarm toggles the alarm on
  ex.pluto_tick(Date.UTC(2026, 8, 1, 12, 0, 30));
  on = onNow();
  check('SimpleAlarm: Alarm button toggles Bell on',
    on.has('0,16') && on.has('2,20'));

  t = Date.UTC(2026, 8, 2, 0, 0, 0);
  pressLight(); // Light press -> edit (hour), alarm stays 00:00
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('SimpleAlarm edit: HO label, hours 00 blinking',
    on.has('1,13') && on.has('2,13') && on.has('1,15')  // H (pos0)
    && on.has('0,11') && on.has('1,11')                 // O (pos1)
    && on.has('2,19') && on.has('0,19')                 // hour tens 0 (shown in edit)
    && on.has('2,20') && on.has('2,21') && on.has('1,21') && !on.has('1,20')); // ones 0

  pressAlarm(); pressAlarm(); // hour 00 -> 02
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('SimpleAlarm edit: hour 02',
    on.has('2,20') && on.has('0,20') && on.has('1,20')  // ones 2 (A,E,G)
    && !on.has('1,21') && !on.has('1,17')               // C,F of 2 off
    && on.has('0,22'));                                 // minutes still 00

  pressLight(); // -> Minute
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('SimpleAlarm edit: MI label, hour 02 steady',
    on.has('0,13') && on.has('1,13') && on.has('2,13') && on.has('1,14') // M (pos0)
    && on.has('0,12') && on.has('1,12')                 // I (pos1)
    && on.has('0,20') && on.has('1,20')                 // hour ones 2 steady
    && on.has('0,22') && on.has('2,22'));               // minute tens 0

  for (let i = 0; i < 5; i++) pressAlarm(); // minute 00 -> 05
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('SimpleAlarm edit: minute 05',
    on.has('2,1') && on.has('0,0') && on.has('1,1') && !on.has('1,0')); // ones 5 (A,D,G; E off)

  pressLight(); // Light on minute -> exit edit
  ex.pluto_tick(Date.UTC(2026, 8, 2, 0, 1, 30));
  on = onNow();
  check('SimpleAlarm view: alarm 02:05, Bell on',
    !on.has('2,19') && on.has('2,20') && on.has('0,20') && on.has('1,20') // hour 02 (tens blank)
    && on.has('0,22') && !on.has('1,23')                 // minute tens 0
    && on.has('2,1') && on.has('0,1') && on.has('1,1')   // minute ones 5 (A,C,G)
    && on.has('0,16'));                                  // Bell on

  // --- SimpleAlarm fires at the top of the minute (second == 0) ---
  goTo('simple_clock'); // Mode -> SimpleClock
  const nSAF = beeps.length;
  ex.pluto_tick(Date.UTC(2026, 8, 2, 2, 5, 0));
  faceIdx = idx('simple_alarm'); // the watch auto-switched to the ringing face
  check('SimpleAlarm fires and auto-switches', beeps.length === nSAF + 1);
  ex.pluto_tick(Date.UTC(2026, 8, 2, 2, 5, 30));
  check('SimpleAlarm ring continues within the minute', beeps.length === nSAF + 2);
  ex.pluto_tick(Date.UTC(2026, 8, 2, 2, 6, 30)); // still ringing (window to 02:07)
  on = onNow();
  check('SimpleAlarm ring shows the current time in full (02:06:30)',
    on.has('2,20') && on.has('0,20') && on.has('1,20')  // hour ones 6
    && on.has('0,22') && !on.has('1,23')                 // minute tens 0
    && on.has('2,1') && !on.has('2,10')                  // minute ones 6 (B off)
    && on.has('0,4') && on.has('2,4')                    // seconds 30
    && on.has('0,16'));                                  // Bell blinks on (ms<500)
  ex.pluto_tick(Date.UTC(2026, 8, 2, 2, 6, 30, 600)); // still ringing, ms=600
  check('SimpleAlarm Bell blinks off at 500ms into the second',
    !onNow().has('0,16') && beeps.length === nSAF + 4);
  ex.pluto_tick(Date.UTC(2026, 8, 2, 2, 7, 0));
  check('SimpleAlarm ring auto-stops after 2 minutes', beeps.length === nSAF + 4);
  on = onNow();
  check('SimpleAlarm back on view after the ring (alarm 02:05, seconds hidden)',
    on.has('0,13') && on.has('2,13') && on.has('1,15')
    && on.has('2,11') && on.has('1,12')
    && on.has('0,16') && on.has('2,20')
    && !on.has('0,4') && !on.has('2,4'));

  // --- chord (Alarm + Light) re-seeds from the current time, seconds dropped ---
  t = Date.UTC(2026, 8, 2, 13, 37, 45);
  chordPress(2, 0); // Alarm + Light -> seed 13:37, enter edit (hour)
  const nSeed = beeps.length;
  ex.pluto_tick(Date.UTC(2026, 8, 2, 13, 37, 50)); // second 50 -> no mid-minute fire
  check('SimpleAlarm: seeded alarm does not fire mid-minute', beeps.length === nSeed);
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('SimpleAlarm: chord seeds current time (13:37, HO)',
    on.has('1,13') && on.has('2,13') && on.has('1,15')  // H
    && on.has('0,11') && on.has('1,11')                 // O
    && on.has('2,19') && on.has('0,19')                 // hour tens 1
    && on.has('2,20') && on.has('2,21') && on.has('1,21') && on.has('1,20') && !on.has('0,20') // ones 3
    && on.has('0,22') && on.has('1,23') && !on.has('2,22') // minute tens 3
    && on.has('2,1') && on.has('2,10') && on.has('0,1') && !on.has('1,1')); // ones 7

  pressAlarm(); pressAlarm(); // hour 13 -> 15
  pressLight(); // -> Minute
  pressAlarm(); // minute 37 -> 38
  pressLight(); // -> exit
  ex.pluto_tick(Date.UTC(2026, 8, 2, 14, 0, 30));
  on = onNow();
  check('SimpleAlarm: nudged alarm to 15:38',
    on.has('2,19') && on.has('0,19')                 // hour tens 1
    && on.has('2,20') && on.has('1,21') && on.has('0,21') && on.has('1,20') && !on.has('2,21') // ones 5 (A,C,D,G; B off)
    && on.has('0,22') && on.has('1,23') && !on.has('2,22') // minute tens 3
    && on.has('0,0') && on.has('1,1') && on.has('2,0')     // minute ones 8 (D,G,F)
    && on.has('0,16'));                             // Bell on

  goTo('simple_clock'); // Mode -> SimpleClock
  const nSAF2 = beeps.length;
  ex.pluto_tick(Date.UTC(2026, 8, 2, 15, 38, 0));
  faceIdx = idx('simple_alarm'); // auto-switched to the ringing face
  check('SimpleAlarm auto-switches and fires at the top of the minute',
    beeps.length === nSAF2 + 1);
  on = onNow();
  check('SimpleAlarm auto-switch view (AL + Bell)',
    on.has('0,13') && on.has('2,13') && on.has('1,15')
    && on.has('2,11') && on.has('1,12')
    && on.has('0,16'));
}

if (ORDER.includes('timer')) {
  // --- Timer: Casio-style countdown (TI), presets via the Alarm+Light chord ---
  goTo('timer'); // Mode -> Timer
  t = Date.UTC(2026, 8, 3, 12, 0, 0);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer view: TI label, preset 1:00, LAP/Bell/H24/PM off',
    on.has('0,13') && on.has('1,14')             // T (pos0)
    && on.has('0,12') && on.has('1,12')          // I (pos1)
    && !on.has('2,13')                           // not an S/M label
    && digitAt(4) === -1                         // hour tens blank (hour 0)
    && digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 1
    && digitAt(8) === 0 && digitAt(9) === 0
    && !on.has('1,10') && !on.has('0,16') && !on.has('2,16') && !on.has('2,17'));

  // Alarm + Light -> next preset (1 -> 3 minutes).
  chordPress(2, 0);
  t = Date.UTC(2026, 8, 3, 12, 0, 0);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: chord -> preset 3:00',
    digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 3
    && digitAt(8) === 0 && digitAt(9) === 0 && !on.has('1,10'));

  // Alarm starts the countdown (LAP on).
  press(2);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Alarm starts the countdown (LAP on, 3:00)',
    on.has('1,10') && digitAt(6) === 0 && digitAt(7) === 3);

  // 1s later -> 2:59.
  t += 1000;
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: countdown decrements (2:59)',
    digitAt(6) === 0 && digitAt(7) === 2 && digitAt(8) === 5 && digitAt(9) === 9);

  // Alarm pauses (LAP off); the remaining time holds.
  press(2);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Alarm pauses (LAP off)', !on.has('1,10'));
  t += 5000; // 5s pass while paused
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: paused countdown holds its time (2:59)',
    digitAt(6) === 0 && digitAt(7) === 2 && digitAt(8) === 5 && digitAt(9) === 9);

  // Alarm resumes; the countdown continues from where it stopped.
  press(2);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Alarm resumes (LAP on)', on.has('1,10'));
  t += 2000;
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: countdown continues after resume (2:57)',
    digitAt(6) === 0 && digitAt(7) === 2 && digitAt(8) === 5 && digitAt(9) === 7);

  // Let it run to zero: the countdown fires a ring.
  const nRing = beeps.length;
  t += 177000; // exhaust the remaining 2:57
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: countdown fires a ring (00:00:00, Bell on, LAP off)',
    on.has('0,13') && on.has('1,14') && on.has('0,16')
    && digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 0
    && digitAt(8) === 0 && digitAt(9) === 0 && !on.has('1,10')
    && beeps.length === nRing + 1);
  // Bell blinks at 2 Hz: off 500ms into the second.
  ex.pluto_tick(t + 600); // ms=600
  check('Timer: Bell blinks at 2Hz during the ring',
    !onNow().has('0,16') && beeps.length === nRing + 2);
  // The ring auto-stops after 120 seconds.
  t += 130000;
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: ring auto-stops after 120s',
    beeps.length === nRing + 2 && !on.has('0,16')
    && digitAt(8) === 0 && digitAt(9) === 0);
  // Alarm after the ring restarts the same duration from the top.
  press(2);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Alarm restarts the finished countdown (3:00, LAP on)',
    on.has('1,10') && digitAt(6) === 0 && digitAt(7) === 3
    && digitAt(8) === 0 && digitAt(9) === 0 && beeps.length === nRing + 2);

  // --- preset cycle: 1 -> 3 -> 5 -> ... -> 60 -> 1 ---
  // (the current preset is 3, so the next chords step 5,7,...,60,1,3)
  const cycle = [5, 7, 10, 15, 20, 30, 40, 60, 1, 3];
  for (const want of cycle) {
    chordPress(2, 0);
    t = Date.UTC(2026, 8, 3, 14, 0, 0);
    ex.pluto_tick(t);
    on = onNow();
    check(`Timer: preset cycle -> ${want} min`,
      digitAt(5) === Math.floor(want / 60) && digitAt(6) === Math.floor((want % 60) / 10)
      && digitAt(7) === (want % 60) % 10
      && digitAt(8) === 0 && digitAt(9) === 0);
  }

  // --- the countdown runs in the background on any face ---
  press(2); // start
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: running again after the preset cycle (LAP on)',
    on.has('1,10') && digitAt(6) === 0 && digitAt(7) === 3);
  goTo('simple_clock'); // Mode -> SimpleClock; the timer keeps counting
  ex.pluto_tick(t);
  const nBack = beeps.length;
  t = Date.UTC(2026, 8, 3, 14, 4, 0); // 4 minutes later, well past the 3:00
  ex.pluto_tick(t); // the timer fires in the background -> auto-switch
  faceIdx = idx('timer'); // the watch switched to the ringing timer
  on = onNow();
  check('Timer: fires from the background and auto-switches',
    on.has('0,13') && on.has('1,14') && on.has('0,16')
    && digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 0
    && digitAt(8) === 0 && digitAt(9) === 0
    && beeps.length === nBack + 1);

  // Any button press silences the ring; in the view Alarm also restarts.
  press(2); // silence + restart running at 3:00
  ex.pluto_tick(t);
  press(2); // pause again for the settings tests
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: paused before editing', !on.has('1,10') && digitAt(6) === 0 && digitAt(7) === 3);

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
  const doublePress = () => {
    t += 2000; ex.pluto_button(2, 1); // Press (first click)
    t += 100;  ex.pluto_button(2, 0); // release
    t += 100;  ex.pluto_button(2, 1); // within 400ms -> Double
    t += 2000; ex.pluto_button(2, 0);
  };
  const holdAlarm = () => {
    t += 2000; ex.pluto_button(2, 1); // Press -> +1
    t += 2000; ex.pluto_button(2, 1); // Hold auto-repeat -> reset
    t += 2000; ex.pluto_button(2, 0);
  };

  // Light enters the settings; the running countdown is paused, SE label.
  pressLight();
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: Light enters settings (SE label, seconds blink on)',
    on.has('0,13') && on.has('2,13') && on.has('0,14')   // S (pos0)
    && on.has('0,11') && on.has('2,11') && on.has('1,12') && on.has('2,12') // E (pos1)
    && digitAt(6) === 0 && digitAt(7) === 3               // minutes steady
    && digitAt(8) === 0 && digitAt(9) === 0               // seconds visible
    && !on.has('1,10'));                                  // paused
  ex.pluto_tick(BLINK_OFF);
  on = onNow();
  check('Timer edit: seconds field blinks (hidden on the off phase)',
    digitAt(8) === -1 && digitAt(9) === -1 && digitAt(6) === 0 && digitAt(7) === 3);

  // Alarm steps the seconds +1; double adds 5; hold resets to 0.
  pressAlarm(); // sec 00 -> 01
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: Alarm steps seconds +1 (00:00:01)',
    digitAt(8) === 0 && digitAt(9) === 1 && digitAt(6) === 0 && digitAt(7) === 3);
  doublePress(); // 01 -> 02 -> 06
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: double press adds 5 to seconds (00:00:06)',
    digitAt(8) === 0 && digitAt(9) === 6);
  holdAlarm(); // +1 (06->07) then hold -> reset to 0
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: hold resets seconds to 0 (00:00:00)',
    digitAt(8) === 0 && digitAt(9) === 0 && digitAt(6) === 0 && digitAt(7) === 3);

  // Light advances to minutes (MI label).
  pressLight(); // SE -> MI
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: MI label, minutes blink, seconds steady',
    on.has('1,13') && on.has('2,14') && on.has('2,13')   // M (pos0)
    && on.has('0,12') && on.has('1,12')                  // I (pos1)
    && digitAt(6) === 0 && digitAt(7) === 3              // minutes visible
    && digitAt(8) === 0 && digitAt(9) === 0);            // seconds steady
  pressAlarm(); // minute 03 -> 04
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: minutes step +1 (00:04:00)',
    digitAt(6) === 0 && digitAt(7) === 4 && digitAt(8) === 0 && digitAt(9) === 0);

  // Light advances to hours (HO label).
  pressLight(); // MI -> HO
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: HO label, hours blink, minutes steady',
    on.has('0,14') && on.has('1,13') && on.has('2,14') && on.has('2,13') && on.has('1,15') // H (pos0)
    && on.has('1,11') && on.has('0,11')                                   // O (pos1)
    && digitAt(4) === 0 && digitAt(5) === 0                               // hours 00 (tens shown)
    && digitAt(6) === 0 && digitAt(7) === 4);                             // minutes steady

  // Hours step up to 23 and wrap to 00 (the 23:59:59 cap).
  for (let i = 0; i < 23; i++) pressAlarm();
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: hours step to 23',
    digitAt(4) === 2 && digitAt(5) === 3 && digitAt(6) === 0 && digitAt(7) === 4);
  pressAlarm(); // 23 -> 0
  ex.pluto_tick(BLINK_ON);
  on = onNow();
  check('Timer edit: hours wrap 23 -> 00',
    digitAt(4) === 0 && digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 4);

  // Light on the hours field exits the settings and arms 4:00.
  pressLight(); // HO -> exit
  t = Date.UTC(2026, 8, 3, 15, 0, 0);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Light on hours exits the settings (view TI, armed 4:00)',
    on.has('0,13') && on.has('1,14') && on.has('0,12') && on.has('1,12')
    && !on.has('2,13')
    && digitAt(5) === 0 && digitAt(6) === 0 && digitAt(7) === 4
    && digitAt(8) === 0 && digitAt(9) === 0 && !on.has('1,10'));

  // Mode away from the middle of a settings session: returning resets to view.
  pressLight(); // enter settings (SE)
  ex.pluto_tick(BLINK_ON);
  goTo('simple_clock'); // Mode -> SimpleClock (leaves the edit mid-way)
  ex.pluto_tick(t);
  goTo('timer'); // Mode x2 -> back to Timer
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: Mode exit from settings returns to view (TI)',
    on.has('0,13') && on.has('1,14') && on.has('0,12') && on.has('1,12') && !on.has('2,13'));

  // Hold Light exits the settings at any point.
  pressLight(); // enter settings (SE)
  ex.pluto_tick(BLINK_ON);
  holdLight(); // hold Light -> exit (and backlight)
  t = Date.UTC(2026, 8, 3, 15, 30, 0);
  ex.pluto_tick(t);
  on = onNow();
  check('Timer: hold Light exits the settings (view TI)',
    on.has('0,13') && on.has('1,14') && on.has('0,12') && on.has('1,12') && !on.has('2,13'));
}

if (fail) process.exit(1);
console.log('ALL OK');
