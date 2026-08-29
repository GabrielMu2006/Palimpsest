# CHRON-009 — Structured Event

## Context
Phase 0 needs persistence-ready structured events rather than strings pretending to be history.
## Scope
Add `sim-events`, stable `EventId`, a versioned envelope, causal/entity references, visibility, significance, metadata, and validation.
## Out of Scope
Gameplay catalogs, NLG, claims/beliefs, SQLite, retention, and replay.
## Dependencies
CHRON-004 and CHRON-005.
## Files Modified / Allowed
Workspace/core manifests and exports; `crates/sim-events/**`; `docs/EVENT_MODEL.md`; ADR-0006; this task.
## API Contract
Version 1 JSON-compatible structured envelope; only stable IDs cross persistence boundaries; invalid schemas/references fail deserialization.
## Tests
Stable IDs, duplicate/self references, significance bounds, serialization round trip, payload-size sanity, and workspace checks.
## Benchmark
Representative JSON payload-size baseline; throughput belongs to CHRON-016.
## Definition of Done
No event truth uses `Vec<String>`; the schema is versioned, validated, causal, persistence-ready, and independent of ECS/Godot/NLG.

