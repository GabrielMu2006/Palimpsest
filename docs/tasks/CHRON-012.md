# CHRON-012 — Snapshot Prototype

## Context
Phase 0 needs versioned restorable state without persisting ECS handles or Scheduler heap internals.
## Scope
Versioned domain snapshot, bincode encoding, zstd compression, stable entities, allocator progress, pending-work DTO, restore validation, size/time benchmark.
## Out of Scope
Final save compatibility, replay UI, `.world` packaging, deltas, and migrations.
## Dependencies
CHRON-004, CHRON-005, CHRON-006, CHRON-009.
## Files Modified / Allowed
Storage crate/dependencies; ADR-0009; snapshot report; performance docs; this task.
## API Contract
Magic + zstd(bincode v2 serde data); schema version 1; only stable domain IDs; validation on encode/decode.
## Tests
Round trip, invalid magic/corruption, dangling work, duplicate IDs, allocator reuse, workspace checks.
## Benchmark
10K entity snapshot raw/compressed size and encode/decode time.
## Definition of Done
Restored domain state equals source, runtime handles are absent, corrupt/incompatible data fails explicitly, and M5 size/time results are recorded.

