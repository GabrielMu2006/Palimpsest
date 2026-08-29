# CHRON-003 — Godot-Rust Bridge

## Context
Phase 0 must prove Godot can load Rust and consume a Rust-produced render view model without owning simulation truth.
## Scope
godot-rust GDExtension crate, macOS library descriptor, presentation bridge class, one typed snapshot call, load/runtime validation, overhead baseline.
## Out of Scope
Tile renderer, full simulation API, Godot-owned state, metrics overlay, and gameplay.
## Dependencies
CHRON-001, CHRON-002, CHRON-004, CHRON-005.
## Files Modified / Allowed
Workspace manifest/lock; `crates/godot-bridge/**`; Godot extension/script/scene and ignored build output; ADR-0007; bridge report; this task.
## API Contract
Rust returns presentation DTOs; Godot never mutates authoritative state. Unsafe is permitted only for the required ExtensionLibrary marker.
## Tests
Rust checks, GDScript validation, extension load, headless preflight, runtime snapshot assertions, zero Godot runtime errors.
## Benchmark
Ten samples of 100,000 GDScript-to-Rust scalar `ping(i64) -> i64` calls, with a
matching GDScript loop baseline subtracted. Results are recorded in
`docs/reports/CHRON-003_GODOT_RUST_BRIDGE.md`.
## Definition of Done
Godot loads the dylib and validates a Rust-produced snapshot while core remains headless and unsafe-free outside the FFI marker. **Complete.**
