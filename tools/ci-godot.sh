#!/bin/sh
set -eu

godot_binary=${PALIMPSEST_GODOT:-/Users/gabrielmu/Applications/Godot.app/Contents/MacOS/Godot}
if [ ! -x "$godot_binary" ]; then
    echo "Godot binary is not executable: $godot_binary" >&2
    exit 1
fi

cargo build -p palimpsest-godot-bridge
mkdir -p apps/macos-godot/bin
cp target/debug/libpalimpsest_godot_bridge.dylib \
    apps/macos-godot/bin/libpalimpsest_godot_bridge.dylib

godot_log=$(mktemp)
"$godot_binary" --headless --path apps/macos-godot --quit-after 30 2>&1 | tee "$godot_log"
if grep -E 'SCRIPT ERROR|ERROR:|CRASH' "$godot_log" >/dev/null 2>&1; then
    rm -f "$godot_log"
    echo "Godot smoke test emitted an error" >&2
    exit 1
fi
if ! grep 'Initialize godot-rust' "$godot_log" >/dev/null 2>&1; then
    rm -f "$godot_log"
    echo "Godot did not initialize the Rust GDExtension" >&2
    exit 1
fi
rm -f "$godot_log"
