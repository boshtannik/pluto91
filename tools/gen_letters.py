#!/usr/bin/env python3
"""Generate the weekday-letter segment tables from letters/letters.json.

The F-91W weekday (mode) digits are data-driven so the user can define which
segments light for each character by editing letters/letters.json (best done
visually in the emulator/letters.html editor). This script compiles that file
into a Rust table used by watch-core's `set_char`.

Two modes:

  python3 tools/gen_letters.py --init
      (Re)generate letters/letters.json from the original Sensor Watch logic
      that used to live in font.rs (CHARACTERS + FONT + CHAR_BIT7 +
      CHAR_FUNKY). Run once to bootstrap the file; after that edit the JSON.

  python3 tools/gen_letters.py
      Read letters/letters.json, write display_map/letters.rs. Characters not
      present in the JSON render as blank (no segments).
"""

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JSON_OUT = os.path.join(ROOT, "letters", "letters.json")
RS_OUT = os.path.join(ROOT, "display_map", "letters.rs")

# ---- Original Sensor Watch character set (bit 0 = A .. bit 6 = G, bit 7 = ninth)
# Index 0 = ' ' (0x20) .. 94 = '~' (0x7e).
CHARACTERS = bytes.fromhex(
    "00 60 22 63 2d 00 44 20 39 0f c0 70 04 40 40 12"
    "3f 06 5b 4f 66 6d 7d 07 7f 6f 00 00 58 48 4c 53"
    "ff 77 7f 39 3f 79 71 3d 76 89 0e 75 38 b7 37 3f"
    "73 67 f7 6d 81 3e 3e be 7e 6e 1b 39 24 0f 23 08"
    "02 5f 7c 58 5e 7b 71 6f 74 10 42 75 30 b7 54 5c"
    "73 67 50 6d 78 62 1c be 7e 6e 1b 16 36 34 01"
)

# A..G glass coordinates for the two weekday positions.
FONT = {
    0: [(0, 13), (1, 13), (2, 13), (2, 15), (2, 14), (0, 14), (1, 15)],
    1: [(0, 11), (1, 11), (1, 11), (2, 11), (1, 12), (1, 12), (2, 12)],
}
CHAR_BIT7 = {0: (1, 14), 1: (0, 12)}
CHAR_FUNKY = {0: (0, 15), 1: (0, 12)}

# Unique (com, seg) per position, in a fixed display order. Used for clearing
# and for the editor's clickable segment list.
POSITIONS = {
    0: [
        {"name": "top", "com": 0, "seg": 13},
        {"name": "upper-left", "com": 1, "seg": 13},
        {"name": "upper-right", "com": 2, "seg": 13},
        {"name": "middle", "com": 2, "seg": 15},
        {"name": "lower-left", "com": 2, "seg": 14},
        {"name": "lower-right", "com": 0, "seg": 14},
        {"name": "bottom", "com": 1, "seg": 15},
        {"name": "center-vertical", "com": 1, "seg": 14},
        {"name": "lower-left-hook", "com": 0, "seg": 15},
    ],
    1: [
        {"name": "top", "com": 0, "seg": 11},
        {"name": "right-vertical", "com": 1, "seg": 11},
        {"name": "middle", "com": 2, "seg": 11},
        {"name": "left-vertical", "com": 1, "seg": 12},
        {"name": "bottom", "com": 2, "seg": 12},
        {"name": "ninth", "com": 0, "seg": 12},
    ],
}


def normalize(ch: str, pos: int) -> str:
    """Mirror of the old display.rs character normalisation."""
    if ch == "u":
        ch = "v"
    elif ch == "j":
        ch = "J"
    if pos == 1:
        ch = {"a": "A", "o": "O", "i": "l", "n": "N", "r": "R", "d": "D",
              "v": "U", "V": "U", "u": "U", "b": "B", "c": "C"}.get(ch, ch)
    elif ch == "R":
        ch = "r"
    return ch


def render(ch: str, pos: int):
    """Old set_char rendering: list of [com, seg] for one char at one position."""
    segs = CHARACTERS[ord(ch) - 0x20]
    out = []
    for i in range(7):
        if segs & (1 << i):
            out.append(list(FONT[pos][i]))
    bit7 = CHAR_BIT7[pos]
    funky = CHAR_FUNKY[pos]
    funky_used = ch in ("B", "D", "@")
    if bit7 == funky:
        if (segs & 0x80) or funky_used:
            out.append(list(bit7))
    else:
        if segs & 0x80:
            out.append(list(bit7))
        if funky_used:
            out.append(list(funky))
    if pos == 1 and ch == "T":
        out.append([1, 12])
    seen = set()
    dedup = []
    for s in out:
        t = tuple(s)
        if t not in seen:
            seen.add(t)
            dedup.append(s)
    return dedup


def chars_from_logic():
    """{'A': {'0': [[...], ...], '1': [[...], ...]}, ...} for every printable char."""
    letters = {}
    for code in range(0x20, 0x7F):
        ch = chr(code)
        letters[ch] = {
            "0": render(normalize(ch, 0), 0),
            "1": render(normalize(ch, 1), 1),
        }
    return letters


def init_json() -> None:
    doc = {
        "positions": {str(p): POSITIONS[p] for p in (0, 1)},
        "letters": chars_from_logic(),
    }
    os.makedirs(os.path.dirname(JSON_OUT), exist_ok=True)
    with open(JSON_OUT, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, ensure_ascii=False, indent=1, sort_keys=True)
        fh.write("\n")
    print(f"wrote {JSON_OUT}")


def load_letters():
    with open(JSON_OUT, encoding="utf-8") as fh:
        doc = json.load(fh)
    positions = {int(p): segs for p, segs in doc["positions"].items()}
    letters = doc["letters"]
    # validate
    valid = {p: {(s["com"], s["seg"]) for s in segs} for p, segs in positions.items()}
    for ch, defs in letters.items():
        for p, lst in defs.items():
            for c, s in lst:
                if (c, s) not in valid[int(p)]:
                    print(f"warning: {ch!r} pos {p} uses unknown segment ({c},{s})",
                          file=sys.stderr)
    return positions, letters


def gen_rust() -> None:
    positions, letters = load_letters()
    lines = [
        "// Generated by tools/gen_letters.py. Do not edit by hand.",
        "// Source of truth: letters/letters.json (edit with emulator/letters.html).",
        "",
        "/// Segments to light for each printable character at each weekday",
        "/// position. Index = (ch - 0x20), 0x20..0x7e. Empty list = blank.",
        "pub const LETTER_SEGS: [[&[(i8, i8)]; 2]; 95] = [",
    ]
    for code in range(0x20, 0x7F):
        ch = chr(code)
        defs = letters.get(ch)
        if defs is None:
            entry = "[ &[], &[] ],"
        else:
            parts = []
            for p in ("0", "1"):
                lst = ["(%d, %d)" % (c, s) for c, s in defs.get(p, [])]
                parts.append("&[%s]" % (", ".join(lst)))
            entry = "[ %s ]," % (", ".join(parts))
        lines.append("    // %s (0x%02x)" % (repr(ch).ljust(3), code))
        lines.append("    %s" % entry)

    lines.append("];")
    lines.append("")
    lines.append("/// Every segment that exists on each weekday position, used to")
    lines.append("/// clear a letter before drawing the next one.")
    lines.append("pub const LETTER_POS_SEGS: [&[(i8, i8)]; 2] = [")
    for p in (0, 1):
        lst = ["(%d, %d)" % (s["com"], s["seg"]) for s in positions[p]]
        lines.append("    &[%s]," % (", ".join(lst)))
    lines.append("];")
    lines.append("")

    os.makedirs(os.path.dirname(RS_OUT), exist_ok=True)
    with open(RS_OUT, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines))
    print(f"wrote {RS_OUT}")


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] == "--init":
        init_json()
    else:
        gen_rust()
    return 0


if __name__ == "__main__":
    sys.exit(main())
