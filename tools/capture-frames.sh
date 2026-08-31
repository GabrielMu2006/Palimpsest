#!/usr/bin/env bash
set -euo pipefail
out=${1:-docs/reports/data/chron-031-frames.json}; if (($# > 0)); then shift; fi
skip_build=0; godot_args=(--path apps/macos-godot --)
for arg in "$@"; do
    if [[ "$arg" == "--skip-build" ]]; then skip_build=1; else godot_args+=("$arg"); fi
done
godot_binary=${PALIMPSEST_GODOT:-/Users/gabrielmu/Applications/Godot.app/Contents/MacOS/Godot}
[[ -x "$godot_binary" ]] || { echo "Godot binary is not executable: $godot_binary" >&2; exit 1; }
if ((skip_build == 0)); then
    cargo build --release --locked -p palimpsest-godot-bridge
    mkdir -p apps/macos-godot/bin
    cp target/release/libpalimpsest_godot_bridge.dylib apps/macos-godot/bin/libpalimpsest_godot_bridge.dylib
fi
case "$out" in /*) output_path=$out ;; *) output_path=$(pwd)/$out ;; esac
mkdir -p "$(dirname "$output_path")"
temp_output=$(mktemp "$(dirname "$output_path")/.chron-031-capture.XXXXXX")
log=$(mktemp "${TMPDIR:-/tmp}/palimpsest-capture.XXXXXX")
cleanup() { rm -f "$temp_output"; }; trap cleanup EXIT
set +e
"$godot_binary" "${godot_args[@]}" "--capture-json=$temp_output" >"$log" 2>&1
status=$?; set -e
if ((status != 0)); then echo "frame capture failed (exit $status); diagnostic log: $log" >&2; cat "$log" >&2; exit "$status"; fi
if grep -E 'SCRIPT ERROR|ERROR:|CRASH' "$log" >/dev/null 2>&1; then echo "frame capture emitted an engine error; diagnostic log: $log" >&2; cat "$log" >&2; exit 1; fi
if [[ ! -s "$temp_output" ]]; then echo "capture report missing or empty; diagnostic log: $log" >&2; cat "$log" >&2; exit 1; fi
if ! ruby -rjson -e 'd=JSON.parse(File.read(ARGV[0])); abort unless d.is_a?(Hash) && d.fetch("records").is_a?(Array) && d.fetch("records").length >= 300' "$temp_output"; then
    echo "capture report is invalid or has fewer than 300 records; diagnostic log: $log" >&2; cat "$log" >&2; exit 1
fi
mv "$temp_output" "$output_path"; trap - EXIT; rm -f "$log"; cat "$output_path"
