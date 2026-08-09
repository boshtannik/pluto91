#!/usr/bin/env python3
"""Embed letters/letters.json into the segment editor so it works from file://.

Replaces the block between the /*__EMBEDDED_START__*/ and /*__EMBEDDED_END__*/
markers inside the editor's JS with a copy of letters/letters.json. Writes both
the source emulator/letters.html and the served emulator/build/letters.html.

Usage: python3 tools/embed_letters.py
"""

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "emulator", "letters.html")
DATA = os.path.join(ROOT, "letters", "letters.json")
OUT = os.path.join(ROOT, "emulator", "build", "letters.html")

with open(DATA, encoding="utf-8") as f:
    payload = json.load(f)

with open(SRC, encoding="utf-8") as f:
    html = f.read()

pattern = re.compile(r"/\*__EMBEDDED_START__\*/(.*?)/\*__EMBEDDED_END__\*/", re.S)
if not pattern.search(html):
    sys.exit("marker /*__EMBEDDED_START__*/ not found in " + SRC)

literal = json.dumps(payload, indent=1, ensure_ascii=False)
html = pattern.sub(
    lambda m: "/*__EMBEDDED_START__*/" + literal + "/*__EMBEDDED_END__*/", html
)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(SRC, "w", encoding="utf-8") as f:
    f.write(html)
with open(OUT, "w", encoding="utf-8") as f:
    f.write(html)

print("embedded %d letters into %s" % (len(payload["letters"]), SRC))
print("wrote %s" % OUT)
