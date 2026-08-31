# ADR-0023: Phase 1 Render Snapshot DTO Contract

> Current supplement: [ADR-0025](ADR-0025-kernel-repair-completion.md).
> Historical decisions below are retained; use the supplement for V2 boundary repairs.

- Status: Accepted — approved by the product owner with CHRON-029 on 2026-08-31
- Date: 2026-08-31
- Decision owners: Product owner
- Task: CHRON-029 in `docs/tasks/CHRON-029.md`; semantics fixed by
  P1-REMAINING D3 in `docs/PHASE_1_REMAINING_EXECUTION.md`
- Extends: ADR-0002, ADR-0007, ADR-0012, ADR-0015; does not supersede the
  Master Spec or define a durable save format

## Context

`MASTER_SPEC.md` §8 and ADR-0007 require Godot to ask "where is Person N" and
receive a presentation snapshot, never to own simulation truth. Phase 0 shipped
a tiny hand-built `VarDictionary` proof; Phase 1 needs a real, typed,
immutable, versioned DTO that batches the whole micro world — terrain, up to
100 persons, their current actions, and observable metrics — in one read from
the `WorldKernel` (CHRON-028, ADR-0022). It must use only stable `EntityId`,
never ECS handles or scheduler tokens, and be built strictly from the kernel's
complete committed boundary so a caller cannot inject a different `now`.

## Decision

### 1. Shape and identity

`RenderSnapshot` (new module `crates/sim-core/src/render.rs`) is a read-only,
serde-serializable DTO in simulation domain (not in the Godot crate, per
ADR-0007/0017):

```rust
pub const RENDER_SCHEMA_VERSION: u16 = 1;

pub struct RenderSnapshot {
    schema_version: u16,
    sim_second: SimInstant,
    terrain: TerrainBatch,
    persons: Vec<PersonRender>,   // sorted by EntityId ascending
    metrics: RenderMetrics,
}

pub struct TerrainBatch { width, height, cells: Vec<TerrainKind> } // row-major 128×128
pub struct PersonRender {
    person_id: EntityId,
    tile: LocalCoord,
    action: ActionKind,
    action_target: Option<LocalCoord>,
    action_state: ActionState,
}
pub struct RenderMetrics {
    person_count: usize,
    scheduler_queue_depth: usize,
    events_committed: u64,
    events_buffered: usize,
    buffer_rotations: u64,
}
```

Invariants:

- The DTO carries stable `EntityId` only; no `bevy_ecs::Entity`, no
  `ScheduleToken`, no runtime reference, no interior mutability, and no `&mut`
  accessor.
- Every `PersonRender.person_id` is non-zero and unique within the batch; the
  batch is sorted ascending by `EntityId`.
- `TerrainBatch.cells.len() == LOCAL_GRID_CELL_COUNT`.
- `sim_second` comes from `WorldKernel::now` at build time; the constructor
  takes no caller-supplied instant.
- Schema version identifies the transient bridge contract; no backward-compatible
  decode or save compatibility is promised in Phase 1.

### 2. Construction and validation

`RenderSnapshot::from_kernel(&WorldKernel)` is the only constructor. It reads
the kernel's committed `now()`, terrain, persons (identity/tile/action), and
metrics; it never mutates the kernel and is a one-shot view of the last fully
committed boundary.

Deserialization (for diagnostics only) re-validates: schema version, cell
count, unique/non-zero person ids, and that every person id is non-zero.
Import values are never written back into the world (ADR-0022/0023 boundary).

### 3. Metrics semantics

`RenderMetrics` is presentation-observable kernel state: scheduler queue depth
(from the action runtime), committed/buffered event counts, and person count.
Client-side fields the snapshot cannot provide are labelled unavailable by the
presenter; the DTO does not invent a value or substitute a measured zero for an
unmeasured one. Wall-clock time and RSS never appear in the DTO.

## Consequences

- CHRON-030 (worker) publishes `Arc<RenderSnapshot>`; CHRON-031 (Godot)
  converts it in `godot-bridge` without exposing a mutator.
- The snapshot is a transient render view, not a save; persistence decisions
  remain under ADR-0009/0016.
- `schema_version` bump requires an explicit ADR; a mismatch is a runtime
  diagnostic error, never a silent decode.

## Rejected / Deferred Alternatives

- Caller-supplied `now` in the builder: rejected; it would let presentation
  fabricate a boundary the kernel did not commit (P1-REMAINING D3).
- Passing ECS handles or scheduler tokens to the client: rejected; ADR-0002/0007.
- Store a full `Vec<EventRecord>` per snapshot or expose interior mutability:
  deferred; Phase 1 keeps a bounded metrics batch.
- Backward-compatible snapshot decode / save format: deferred; out of scope
  (ADR-0009, source completeness).

## Task Completion / Acceptance Gate

- Dependencies: CHRON-028 kernel (ADR-0022) and the accepted ADRs governing
  identity, bridge, and coordinates.
- Files: this ADR plus CHRON-029's allowed implementation/report surface.
- Tests and benchmark: per `docs/tasks/CHRON-029.md`, including version,
  structural bounds, unique/non-zero identity, immutability, headless boundary,
  and fidelity to the committed tick; size/throughput measured for 100 persons.
- DoD: the semantics above hold; no save compatibility promise is made.
