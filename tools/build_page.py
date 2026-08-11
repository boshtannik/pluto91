#!/usr/bin/env python3
"""Build the emulator page.

Takes the Sensor Watch shell (emulator/shell.html), which already contains the
F-91W skin, the skin switcher and the button wiring, and replaces the
emscripten `{{{ SCRIPT }}}` placeholder with the glue script that loads the
Rust WASM module. The Rust firmware exposes:

  * pluto_init()                 - create the runtime and boot into the clock
  * pluto_tick(ms_epoch)         - periodic tick (called ~4x/sec)
  * pluto_button(id, down)       - 0=Light 1=Mode 2=Alarm; 1=pressed 0=released

The JS provides the imports the Rust side expects (js_clear, js_seg, js_now,
js_panic, js_backlight, js_beep) and drives the SVG segments by setting
style.opacity on the `[data-com][data-seg]` elements.

Usage: python3 tools/build_page.py <shell.html> <out.html>
"""

import os
import sys

GLUE = r"""
<script>
(function () {
  'use strict';
  var outputEl = document.getElementById('output');
  function log(text) {
    console.log(text);
    if (outputEl) {
      outputEl.value += text + '\n';
      outputEl.scrollTop = outputEl.scrollHeight;
    }
  }

  // WebAudio beep for the buzzer. `delayMs` is an offset from "now", so a
  // melody is just several scheduled tones. Active tones are remembered so
  // `stop_melody` can cut them short.
  var audioCtx = null;
  var activeTones = [];
  function beep(freq, ms, delayMs) {
    if (!audioCtx) {
      try { audioCtx = new (window.AudioContext || window.webkitAudioContext)(); }
      catch (e) { return; }
    }
    if (audioCtx.state === 'suspended') audioCtx.resume();
    var t0 = audioCtx.currentTime + delayMs / 1000;
    var osc = audioCtx.createOscillator();
    var gain = audioCtx.createGain();
    osc.type = 'square';
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0.05, t0);
    gain.gain.setValueAtTime(0.001, t0 + ms / 1000);
    osc.connect(gain);
    gain.connect(audioCtx.destination);
    osc.start(t0);
    osc.stop(t0 + ms / 1000);
    var tone = { osc: osc, gain: gain, t0: t0, end: t0 + ms / 1000 };
    activeTones.push(tone);
    osc.onended = function () {
      activeTones = activeTones.filter(function (e) { return e !== tone; });
    };
  }
  function stopMelody() {
    activeTones.forEach(function (tone) {
      if (audioCtx.currentTime < tone.end) {
        tone.gain.gain.setValueAtTime(0, audioCtx.currentTime);
        tone.osc.stop(audioCtx.currentTime);
      }
    });
    activeTones = [];
  }

  // Local wall-clock time expressed as if it were UTC (matches the real RTC,
  // which is set to local time). `clockOffset` shifts it when the calendar's
  // settings write a new wall clock, so the emulator keeps ticking from there.
  function nowLocal() {
    return Date.now() - new Date().getTimezoneOffset() * 60000;
  }
  var clockOffset = 0;

  var imports = {
    env: {
      // The firmware's DateTime is timezone-agnostic: it renders whatever
      // civil time it is given. The real RTC is set to *local* wall-clock
      // time, so the emulator feeds the same thing: local time expressed
      // as if it were UTC.
      js_now: function () {
        return nowLocal() + clockOffset;
      },
      // The calendar face's settings wrote a new wall clock: shift the
      // baseline so js_now returns `ms` now and keeps ticking from there.
      js_set_time: function (ms) {
        clockOffset = ms - nowLocal();
      },
      js_clear: function () {
        document.querySelectorAll('[data-com][data-seg]')
          .forEach(function (e) { e.style.opacity = 0; });
      },
      js_seg: function (com, seg, on) {
        var els = document.querySelectorAll(
          '[data-com="' + com + '"][data-seg="' + seg + '"]');
        for (var i = 0; i < els.length; i++) {
          els[i].style.opacity = on ? 1 : 0;
        }
      },
      js_backlight: function (on) {
        var el = document.getElementById('light');
        if (el) el.style.opacity = on ? 1 : 0;
      },
      js_beep: beep,
      js_stop_melody: stopMelody,
      js_panic: function () { log('Rust panicked! See console for details.'); }
    }
  };

  var instance = null;
  fetch('watch.wasm?v={{{ WASM_VERSION }}}')
    .then(function (r) { return r.arrayBuffer(); })
    .then(function (buf) { return WebAssembly.instantiate(buf, imports); })
    .then(function (result) {
      instance = result.instance;
      instance.exports.pluto_init();
      // pluto's tick fires at 4 Hz (see svc/main.c's prescaler).
      setInterval(function () { instance.exports.pluto_tick(imports.env.js_now()); }, 250);
      log('Pluto emulator ready. W/S/D, arrow keys or the buttons on the watch.');
    })
    .catch(function (e) { log('failed to load watch.wasm: ' + e); });

  function press(id) {
    if (instance) instance.exports.pluto_button(id, 1);
  }
  function release(id) {
    if (instance) instance.exports.pluto_button(id, 0);
  }

  // While a button is held (mouse), keep reporting it as pressed so the Rust
  // gesture scanner can fire hold auto-repeat (HOLD_DELAY_MS/REPEAT_MS) and
  // double-click events. Keyboard relies on the OS key repeat instead.
  var holdTimer = {};
  function startHold(id) {
    if (!holdTimer[id]) holdTimer[id] = setInterval(function () { press(id); }, 250);
  }
  function stopHold(id) {
    if (holdTimer[id]) { clearInterval(holdTimer[id]); holdTimer[id] = null; }
    release(id);
  }

  // SVG buttons: btn1 = Light, btn2 = Mode, btn3 = Alarm
  [['btn1', 0], ['btn2', 1], ['btn3', 2]].forEach(function (pair) {
    var el = document.getElementById(pair[0]);
    if (el) {
      el.addEventListener('mousedown', function () { press(pair[1]); startHold(pair[1]); });
      el.addEventListener('mouseup', function () { stopHold(pair[1]); });
      el.addEventListener('mouseleave', function () { stopHold(pair[1]); });
    }
  });

  var KEY_IDS = {
    w: 0, W: 0, s: 1, S: 1, d: 2, D: 2,
    ArrowUp: 0, ArrowDown: 1, ArrowRight: 2
  };
  window.addEventListener('keydown', function (e) {
    if (KEY_IDS[e.key] !== undefined) press(KEY_IDS[e.key]);
  });
  window.addEventListener('keyup', function (e) {
    if (KEY_IDS[e.key] !== undefined) release(KEY_IDS[e.key]);
  });
})();
</script>
"""

PLACEHOLDER = "{{{ SCRIPT }}}"


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    with open(src, encoding="utf-8") as fh:
        page = fh.read()
    if PLACEHOLDER not in page:
        print(f"error: {PLACEHOLDER} not found in {src}", file=sys.stderr)
        return 1
    # Cache-bust the wasm URL: embed the wasm file's mtime so the browser
    # always fetches the freshly built module (python's http.server sends no
    # Cache-Control, and heuristic caching keeps stale wasm alive forever).
    wasm_path = os.path.join(os.path.dirname(os.path.abspath(dst)), "watch.wasm")
    try:
        version = str(int(os.path.getmtime(wasm_path)))
    except OSError:
        version = "0"
    glue = GLUE.replace("{{{ WASM_VERSION }}}", version)
    page = page.replace(PLACEHOLDER, glue)
    with open(dst, "w", encoding="utf-8") as fh:
        fh.write(page)
    print(f"wrote {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
