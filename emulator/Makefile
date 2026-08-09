# Pluto browser emulator.
#
# Builds the same Rust firmware (crates/pluto-emu) to WASM and wraps it
# in the F-91W skin page:
#
#   make -C emulator
#   python3 -m http.server -d emulator/build
#   -> open http://localhost:8000/watch.html
#
# Controls: buttons on the watch (btn1=Light, btn2=Mode, btn3=Alarm) or the
# keyboard (W=Light, S=Mode, D=Alarm, or the arrow keys Up/Down/Right).

CARGO = cargo
BUILD = build
WASM = $(BUILD)/watch.wasm
TARGET = $(BUILD)/watch.html

EMU_CRATE = $(CURDIR)/../crates/pluto-emu
WASM_OUT = $(CURDIR)/../target/wasm32-unknown-unknown/release/pluto_emu.wasm

all: $(TARGET) $(WASM)

$(WASM): $(shell find $(EMU_CRATE)/src ../crates/pluto-core/src ../crates/pluto-faces/src -name '*.rs') ../display_map/display_map.rs ../display_map/letters.rs
	$(CARGO) build --release -p pluto-emu --target wasm32-unknown-unknown
	mkdir -p $(BUILD)
	cp $(WASM_OUT) $(WASM)

$(TARGET): shell.html ../tools/build_page.py
	mkdir -p $(BUILD)
	python3 ../tools/build_page.py shell.html $(TARGET)

clean:
	rm -rf $(BUILD)

.PHONY: all clean
