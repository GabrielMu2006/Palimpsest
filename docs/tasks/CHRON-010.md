# CHRON-010 — Developer Metrics Overlay

## Context
Phase 0 needs a first runtime surface that makes architecture and performance state inspectable without turning Godot into simulation truth.
## Scope
A read-only Godot overlay showing engine FPS/timing/render metrics, tile benchmark state, Rust bridge status/latency, and fields from the Rust Render Snapshot.
## Out of Scope
Rule editing, simulation mutation, profiler history graphs, production visual design, gameplay UI, and unavailable Phase 1 simulation metrics.
## Dependencies
CHRON-003 and CHRON-011.
## Files Modified / Allowed
Godot scene/scripts, this task, and architecture documentation.
## Tests
GDScript and scene validation, headless preflight, live windowed inspection, expected metric count/text, screenshot inspection, and zero runtime diagnostics.
## Benchmark
Not applicable. The overlay displays measured values but is not itself a throughput benchmark.
## Definition of Done
The windowed client visibly presents at least ten read-only metrics, including Rust snapshot provenance and the full tile count, without any authoritative simulation mutation control. **Complete.**
