# CHRON-017 — Headless / Rendered Mode Comparison

## Context
The Phase 0 report requires an evidence-based speed difference between Headless and Rendered modes.
## Scope
One shared deterministic 10K-entity Rust workload, standalone and windowed-Godot release harnesses, ten samples per mode, invariant checks, comparison report, and ADR-0010.
## Out of Scope
Real NPC systems, frame-paced simulation policy, threading architecture, background workers, gameplay, and Phase 1 performance budgets.
## Dependencies
CHRON-003, CHRON-007, CHRON-011, and CHRON-015.
## Files Modified / Allowed
`sim-core`, headless benchmark bin, Godot bridge/client metrics, ADR-0010, this task, architecture documentation, and comparison report.
## API Contract
`sim-core::run_spike_workload` is a temporary deterministic measurement API shared inward by both adapters. The headless runner preserves its existing `run`/metrics/error exports through aliases; Godot receives only a read-only benchmark dictionary. ADR-0010 requires this API to be reviewed or removed before Phase 1.
## Tests
All Rust gates, invalid/valid workload tests, GDScript validation, windowed live result/invariants, exact workload identity, and zero runtime diagnostics.
## Benchmark
Ten release samples of exactly 10,000 scheduled dummy entities through simulation second 1,000 in each mode; report medians, work/s, ratio, and limitations.
## Definition of Done
Both modes execute the same shared Rust function successfully and the report states the measured ratio without treating dummy work as full NPC simulation. **Complete.**
