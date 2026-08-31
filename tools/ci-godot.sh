#!/usr/bin/env bash
set -euo pipefail

godot_binary=${PALIMPSEST_GODOT:-/Users/gabrielmu/Applications/Godot.app/Contents/MacOS/Godot}
if [ ! -x "$godot_binary" ]; then
    echo "Godot binary is not executable: $godot_binary" >&2
    exit 1
fi
cargo build --locked -p palimpsest-godot-bridge
mkdir -p apps/macos-godot/bin
cp target/debug/libpalimpsest_godot_bridge.dylib apps/macos-godot/bin/libpalimpsest_godot_bridge.dylib

godot_log=$(mktemp)
trap 'rm -f "$godot_log"' EXIT
run_checked() {
    # pipefail preserves engine failure; script errors fail even if Godot exits zero.
    "$godot_binary" --headless --path apps/macos-godot "$@" 2>&1 | tee "$godot_log"
    if grep -E 'SCRIPT ERROR|ERROR:|CRASH' "$godot_log"; then
        echo "Godot integration emitted an error" >&2
        exit 1
    fi
    if ! grep -q 'Initialize godot-rust' "$godot_log"; then
        echo "GDExtension did not initialize" >&2
        exit 1
    fi
}
# Builds the script class/import cache in a clean checkout before scene execution.
run_checked --editor --import --quit
run_checked --quit-after 30
run_checked --script res://tests/capture_statistics.gd
run_checked --script res://tests/chron031_integration.gd
# Headless CI cannot verify GPU readback or FPS: native windowed evidence is local.
