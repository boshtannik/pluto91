#!/usr/bin/env bash
# Quick-start the Pluto browser emulator: build + serve + open.
#
#   ./run.sh                 # build, serve on :8000, open the browser
#   PORT=9000 ./run.sh       # custom port
#
# Controls: W=Light, S=Mode, D=Alarm (or the buttons on the watch).

set -euo pipefail
cd "$(dirname "$0")"

# 1. WASM target (needed by `make -C emulator`).
if ! rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
    echo "Adding wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# 2. Build the emulator (WASM + page).
echo "Building the emulator (make -C emulator)..."
make -C emulator

# 3. Serve the page and open the browser.
PORT="${PORT:-8000}"
URL="http://localhost:${PORT}/watch.html"

echo
echo "  Pluto emulator:  ${URL}"
echo "  controls: W=Light  S=Mode  D=Alarm  (or the buttons on the watch)"
echo "  press Ctrl+C to stop."
echo

if command -v xdg-open >/dev/null 2>&1; then
    (sleep 1; xdg-open "$URL") >/dev/null 2>&1 &
fi

exec python3 -m http.server "$PORT" -d emulator/build
