# CHRON-006 — Extensible Scheduler

## Context

Palimpsest cannot scan every entity every simulation second. Phase 0 needs a deterministic due-time queue that allows systems to schedule only necessary future work and exposes queue health to developer metrics.

## Scope

- Add a generic, Godot-independent `palimpsest-sim-scheduler` crate.
- Schedule payloads at absolute `SimInstant` values.
- Guarantee due-time ordering and FIFO ordering for equal due instants.
- Support cancellation, rescheduling, stale-node compaction, and due-work popping.
- Expose live-entry, queue-node, and stale-node metrics.
- Add a repeatable release benchmark for enqueue/dequeue throughput.
- Record ordering and lifecycle policy in ADR-0004.

## Out of Scope

- Executing callbacks or systems inside the Scheduler.
- ECS integration, entity scans, NPC AI, Simulation LOD policy, threads, or async runtime.
- Persistent Scheduler snapshots; CHRON-012 will define the persisted representation.
- Calendar cadence, Godot frame scheduling, or wall-clock timers.

## Dependencies

- CHRON-004 Stable EntityId complete, without coupling schedule tokens to entity identity.
- CHRON-005 Simulation Clock complete.

## Files Modified / Allowed

- `Cargo.toml`
- `Cargo.lock`
- `crates/sim-core/Cargo.toml`
- `crates/sim-core/src/lib.rs`
- `crates/sim-scheduler/**`
- `docs/ARCHITECTURE.md`
- `docs/PERFORMANCE.md`
- `docs/adr/ADR-0004-scheduler-contract.md`
- `docs/reports/CHRON-006_SCHEDULER_BASELINE.md`
- `docs/tasks/CHRON-006.md`

## API Contract

- `Scheduler<T>` owns queued payloads but never executes them.
- Earlier due instants pop first; equal due instants use stable insertion order.
- Rescheduling assigns a new equal-time insertion order.
- Cancellation and pop make a token permanently non-live.
- `ScheduleToken` is runtime-local and never substitutes for `EntityId`.
- Stale heap nodes are observable and compactable.

## Tests

- Empty queue and future-work behavior.
- Due-time ordering and equal-time FIFO.
- Idempotent cancellation and rescheduling semantics.
- Caller-controlled reentrant scheduling.
- Stale-node metrics and compaction.
- 100,000-item correctness workload.
- Workspace fmt, Clippy, debug/release tests, docs, and dependency audit.

## Benchmark

- Release-mode enqueue/dequeue measurements at 1K, 10K, and 100K items.
- Ten samples per size after one unreported process warm-up.
- Record median operations/s on the M5 16GB reference machine.
- Record maximum RSS for a 100K-item process and limitations of process-level measurement.

## Definition of Done

- Work pops only when due and in the documented deterministic order.
- Cancellation, rescheduling, and compaction preserve correctness.
- Queue health metrics are available without scanning simulation entities.
- The Scheduler remains headless and independent of ECS, Godot, and LLMs.
- Functional and performance results are reproducible and documented.
- ADR-0004 records public ordering and lifecycle behavior.

