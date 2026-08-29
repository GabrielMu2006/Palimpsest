# CHRON-004 — Stable EntityId

## Context

Persistent simulation identity must survive ECS rebuilds, snapshots, event storage, and client reconnects. Runtime ECS handles are process-local implementation details and cannot be used as permanent identity.

## Scope

- Add the independent `palimpsest-sim-entity` domain crate.
- Define a non-zero `EntityId` backed by `u64` with numeric Serde representation.
- Add a monotonic, serializable allocation prototype with explicit exhaustion.
- Re-export the canonical identity types from `palimpsest-sim-core`.
- Test persistence boundaries using a fake generational runtime handle.
- Record the public identity and allocation contract in ADR-0002.

## Out of Scope

- Selecting or integrating `bevy_ecs`.
- A production ECS runtime lookup map, entity components, NPC data, or lifecycle systems.
- Distributed ID generation, UUIDs, ID recycling, database allocation, or save migrations.
- Godot bridge conversion, Scheduler, Event Store, or Snapshot implementation.

## Dependencies

- CHRON-001 complete.
- Serde for explicit persistence boundaries; Serde JSON is test-only.

## Files Modified / Allowed

- `Cargo.toml`
- `Cargo.lock`
- `crates/sim-core/Cargo.toml`
- `crates/sim-core/src/lib.rs`
- `crates/sim-entity/**`
- `docs/ARCHITECTURE.md`
- `docs/adr/ADR-0002-stable-entity-id.md`
- `docs/tasks/CHRON-004.md`

## API Contract

- `EntityId` is an opaque, copyable, ordered, hashable non-zero `u64` newtype.
- Its serialized representation is a plain non-zero unsigned integer.
- Zero is always invalid and reserved as a sentinel outside the domain type.
- `EntityIdAllocator` allocates monotonically and never recycles an ID.
- Allocator exhaustion is returned as an explicit error.
- Runtime ECS handles are not defined or serializable through this crate.

## Tests

- Size and non-zero invariant.
- Numeric text and Serde round trips, including zero rejection.
- 10,000 sequential allocations are unique and ordered.
- Allocator serialization resumes without ID reuse.
- Restore advancement and `u64::MAX` exhaustion behavior.
- Persistence DTO contains only `EntityId`, not a fake runtime handle.
- Workspace fmt, Clippy with warnings denied, tests, docs, and dependency-tree audit.

## Benchmark

Not applicable. This task introduces no production ECS lookup map or simulation workload. Allocation and lookup performance will be measured with the selected ECS in CHRON-008.

## Definition of Done

- All valid IDs occupy exactly one `u64`; zero cannot be constructed or deserialized.
- Allocation is monotonic, serializable, non-recycling, and explicitly fallible on exhaustion.
- Runtime handles do not appear in the persistence contract or `sim-entity` dependency graph.
- `sim-core` remains headless and independent of Godot and LLMs.
- ADR-0002 records the cross-module identity contract.
- All specified checks pass without skipped or weakened tests.

