# CHRON-029 — Render Snapshot DTO

> Final corrective verification/measurement: [repair V2 report](../reports/P1_KERNEL_REPAIR_V2.md).
> Use [CURRENT_PROGRESS](../CURRENT_PROGRESS.md); the original status below is historical.

> **Status: Implemented 2026-08-31 under the approved CHRON-029 Task; ADR-0023 accepted; schema 2 + repair 2026-08-31 under ADR-0024.**
> This Task was separately approved by the product owner on 2026-08-31 and implemented
> with the ADR-0023 render-snapshot DTO contract; see `docs/reports/CHRON-029_RENDER_SNAPSHOT.md`.
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Define and implement a read-only, immutable, versioned Render Snapshot DTO that batches terrain, person, action, and metric data from the Simulation Core for the Godot presentation layer. It must use only stable `EntityId` (never ECS handles), carry a version, and pass explicit size/version tests. It must not expose ECS internals.

## Context
`MASTER_SPEC.md` §8 and ADR-0007 require Godot to ask "where is Person 8127" and receive a Render Snapshot, never to own simulation truth. Phase 0 shipped a tiny hand-built `VarDictionary` from `godot-bridge` as a proof; Phase 1 needs a real, typed, versioned, immutable DTO that can batch the whole micro world (terrain, up to 100 persons, their current actions, and developer metrics) in one call. This is the contract that keeps Scene Tree state presentation-only and sets the boundary CHRON-031 consumes and that ADR-0007 calls "batched render data."

## Scope
- Define a read-only, serde-serializable Render Snapshot DTO (in Simulation Core domain, not in the Godot crate) that contains:
  - a top-level `schema_version` (u16) and `sim_second` (`SimInstant`);
  - a terrain/local-tile batch (terrain fields + local tile data from CHRON-019..020) sufficient to render tiles;
  - a persons batch: for each person a stable `EntityId`, position/tile, and current `ActionKind`/action state (from CHRON-027);
  - a static ActivitySite batch and read-only Needs values needed by the micro-world display; Godot does not invent these;
  - a metrics batch: scheduler queue depth, active/processed work, events/s or event count, person count, tick/advance counters (from CHRON-028 kernel + sim-debug);
  - no executable mutation methods, no interior mutability, no `bevy_ecs` handle, no `ScheduleToken`, no heap/internal runtime references.
- Provide construction only through a validated snapshot builder from the kernel (CHRON-028) so the DTO is always a faithful, one-shot view of a committed tick boundary.
- Provide an explicit `schema_version` constant for the transient bridge contract. Phase 1 does not define backward-compatible render-snapshot decoding or a save format.
- Keep Godot conversion out of this Task; CHRON-030/031 may add the narrow adapter after this headless DTO is fixed.
- Keep the DTO fully headless and domain-only so it can also be serialized headlessly (harmless for diagnostics), while never being used as persistent save data.

## Out of Scope
- ECS exposure: no `bevy_ecs::Entity`, no runtime handle, no `ScheduleToken`, no World access leaking across the boundary.
- Mutation: the DTO is immutable; presentation cannot call it to change simulation.
- Persistence/save format: CHRON-012/ADR-0009 define snapshots; this DTO is a transient render view, not a save.
- Final art, Godot scenes, rendering implementation, animation, or camera (CHRON-031).
- Utility AI scoring, action selection, or transition logic.
- Economy, resources, production.
- LLM, NLG, war, politics, religion, magic.

## Dependencies
- CHRON-019, CHRON-020, CHRON-021 complete (terrain, local tile, person entity data model provided to the DTO).
- CHRON-028 complete (kernel that produces a committed, tick-bounded snapshot view).
- CHRON-027 complete (current `ActionKind`/action state surfaced per person).
- CHRON-006 scheduler metrics and CHRON-010 developer-metrics conventions for the metrics batch.

## Execution Steps / Readiness

1. Parent records fields, ordering, version, validation/error cases and exact
   builder signature in a render-contract ADR (P1-REMAINING D3).
2. Build only from a complete kernel boundary; no arbitrary caller time, ECS
   handle, or imported diagnostic writes. Add checked private wire decoding.
3. Test bad versions/counts/coordinates/duplicate IDs through both construction
   and decoding; successful round trips alone are insufficient.
4. Create `render_snapshot_bench`, with cold/prepared memory adapters, using §4.
   DTO/tests are suitable Luna leaves only after parent fixes this contract.

## Files Modified / Allowed
- `crates/sim-core/**` (new `render_snapshot` module defining the DTO and its builder).
- `Cargo.toml`, `Cargo.lock` only if a workspace member changes; prefer `sim-core` module.
- ADR-0015 and ADR-0012 govern this DTO and tile-coordinate boundary; ADR-0007 and ADR-0002 govern the Godot and identity boundaries. A new ADR is required only if implementation diverges.
- `docs/reports/CHRON-029_RENDER_SNAPSHOT.md` for recorded size/throughput measurements.
- `docs/tasks/CHRON-029.md`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- A public immutable DTO, e.g. `RenderSnapshot`, with public read-only getters and a top-level:
  - `schema_version() -> u16`
  - `sim_second() -> SimInstant`
  - `terrain() -> &TerrainBatch`
  - `persons() -> &[PersonRender]`
  - `metrics() -> &RenderMetrics`
  - `person_count() -> usize`
- `PersonRender` exposes only stable data: `person_id() -> EntityId`, `tile() -> ...`, `action() -> ActionKind` (plus, if needed, a bounded action-state enum), never an ECS handle.
- `RenderMetrics` exposes only observable counters from the kernel/debug: scheduler queue depth, processed work, events/s or event count, person count, advance counter; no internal pointers.
- `snapshot_from_kernel(&WorldKernel) -> Result<RenderSnapshot, SnapshotViewError>` builds from the kernel's own complete boundary, never a supplied `now`. Incomplete-boundary reads fail explicitly. Diagnostic deserialization uses an equally validated private wire path and cannot mutate/reconstruct the simulation.
- Invariants to document:
  1. `RenderSnapshot` carries stable `EntityId` only; no ECS handle or `ScheduleToken` appears in the DTO or its serde form.
  2. The DTO is immutable after construction (no `&mut` mutators, no interior mutability).
  3. `schema_version` identifies the transient bridge schema; compatibility guarantees require a later explicit ADR.
  4. Every `PersonRender` references a stable `EntityId` that is non-zero and unique within the batch.
  5. The DTO is domain-only and headless; it never depends on Godot and is never used as a save.

## Tests
- Version test: a snapshot exposes the declared `SCHEMA_VERSION`, and the value is included in diagnostic serialization.
- Bound test: a 100-person + full-local-tile snapshot contains exactly the expected bounded tile/person counts; measured serialized bytes are recorded without inventing an unapproved byte budget.
- Identity test: every `PersonRender.person_id()` is non-zero and unique; no encoded tuple contains a `bevy_ecs::Entity`, `ScheduleToken`, or runtime generation.
- Immutability/boundary test: owned snapshot data has no interior mutability or runtime references; mutation remains available only while a private builder constructs the value.
- Headless boundary test: a `RenderSnapshot` serializes/deserializes for diagnostics without referencing Simulation Core mutable state or any Godot type.
- Fidelity test: for a fixed kernel state and tick, the snapshot's `sim_second`, person list, and metrics match the kernel's committed tick; no cross-tick bleed.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests (excluding Godot-specific tests from pure-Rust gates), docs, dependency audit.

## Benchmark
- Build+serialize a Render Snapshot for 100 persons and a full local tile set, release build, ten post-warm-up samples, median reported on M5 16GB.
- Report snapshot build wall-time, serialize wall-time, serialized bytes, bytes/person, and peak RSS delta against a no-snapshot control.
- Record serialized size and hand the measured result to CHRON-031 for the 60 FPS presentation assessment.

## Definition of Done
- A typed, immutable, versioned `RenderSnapshot` DTO batches terrain, persons (with current action), and metrics using stable `EntityId` only.
- The DTO never exposes an ECS handle, `ScheduleToken`, or runtime internal; it has no mutation methods.
- The schema version and structural-bound tests pass; no unsupported compatibility promise is made.
- Runtime snapshots come from the validated kernel builder; private diagnostic decoding enforces the same structural/version rules and is not execution input. Godot conversion belongs to CHRON-030/031.
- The DTO is headless and domain-only and is not used as a persistent save. Size/throughput results are reproducible and documented under ADR-0015.

## Required Completion Report
Report: change summary; commands run; benchmark result (build/serialize time, bytes, bytes/person, RSS delta) with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; list of covered tests (version/size/identity/immutability/boundary/fidelity); known limitations (e.g., DTO is transient and not a save; no final art); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
