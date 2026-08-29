# CHRON-003 Godot-Rust Bridge Result

Date: 2026-08-29

## Environment

- Apple M5 MacBook Air, 10 cores, 16 GB RAM
- macOS 26.6.2
- Godot 4.7.2 stable official, arm64
- godot-rust 0.5.5, API feature 4.7
- Debug Rust dynamic library; Godot headless runtime

## Functional Result

Godot loaded `libpalimpsest_godot_bridge.dylib`, registered
`PalimpsestBridge`, and consumed a Rust-produced render snapshot. Runtime
validation confirmed schema version 1, source `rust`, and stable example
`EntityId` 1. GDA static validation, scene preflight, live runtime inspection,
and a direct ten-frame Godot run completed without runtime diagnostics.

The Simulation Core remains independently runnable and has no Godot dependency.
The bridge exposes presentation data only; the Godot Scene Tree is not
authoritative simulation state.

## Call Overhead

Method measured: `ping(i64) -> i64`. Each sample executed 100,000 calls from a
GDScript loop. A same-shape GDScript assignment loop was measured immediately
before each call loop and subtracted. The figures therefore include GDScript
dispatch, GDExtension marshalling, the Rust method, and return marshalling, but
exclude most loop overhead.

Sorted net samples in nanoseconds per call:

`339.39, 343.18, 351.84, 352.33, 353.37, 354.67, 357.67, 359.80, 364.94, 366.76`

- Reported median: **354.67 ns/call**
- Approximate reciprocal throughput: **2.82 million calls/s**

This scalar microbenchmark is a lower-bound boundary measurement, not a render
snapshot throughput result. Production rendering should still transfer batched
view models instead of making one bridge call per entity.

## Known Issue

One invocation of Godot's `--headless --editor --quit` path crashed with SIGSEGV
after successful extension registration while loading the editor layout. Normal
headless game execution, GDA preflight, and a live windowed game session all
succeeded. This is currently classified as an editor-only integration risk and
will remain visible in the final spike report; recurrence in normal game mode
would change the recommendation.
