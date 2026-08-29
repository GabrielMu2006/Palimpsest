# CHRON-005 — Simulation Clock

## Context

Every scheduled system, event, snapshot, and history record needs a deterministic simulation timeline that is independent of wall-clock time and rendering cadence.

## Scope

- Add the independent `palimpsest-sim-time` crate.
- Define signed integer-second `SimInstant` values.
- Define non-negative integer-second `SimDuration` values.
- Define a serializable, monotonic `SimClock` with checked advancement.
- Re-export the canonical time types from `palimpsest-sim-core`.
- Record the representation and monotonicity contract in ADR-0003.

## Out of Scope

- Calendars, eras, seasons, localized formatting, wall-clock synchronization, or time controls.
- Scheduler queues, system cadence, Simulation LOD, Godot frame timing, or event processing.
- Floating-point or sub-second simulation time.

## Dependencies

- CHRON-001 complete.
- Serde workspace dependency.

## Files Modified / Allowed

- `Cargo.toml`
- `Cargo.lock`
- `crates/sim-core/Cargo.toml`
- `crates/sim-core/src/lib.rs`
- `crates/sim-time/**`
- `docs/ARCHITECTURE.md`
- `docs/adr/ADR-0003-simulation-time.md`
- `docs/tasks/CHRON-005.md`

## API Contract

- `SimInstant(i64)` is an integer number of simulation seconds from an epoch.
- `SimDuration` is an integer number of non-negative simulation seconds.
- `SimClock` never moves backward.
- Addition and clock advancement never wrap; overflow is explicit.
- Serde uses numeric seconds and is independent of Godot or wall time.

## Tests

- Instant and clock serialization round trips.
- Negative duration rejection, including deserialization.
- Monotonic advancement and same-instant behavior.
- Time-reversal rejection without mutation.
- Arithmetic overflow and representable-boundary behavior.
- Workspace fmt, Clippy, debug/release tests, docs, and dependency audit.

## Benchmark

Not applicable. Scheduler throughput is measured in CHRON-006.

## Definition of Done

- Time is represented deterministically as integer seconds.
- Invalid negative durations cannot be constructed or deserialized.
- Clock reversal and overflow return explicit errors without changing state.
- The crate remains headless and independent of Godot, ECS, and LLMs.
- ADR-0003 records the public serialization and monotonicity contract.
- All specified checks pass without skipped or weakened tests.

