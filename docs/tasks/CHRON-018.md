# CHRON-018 — Phase 1 Workspace Boundaries

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

## Context
Phase 0 established a virtual Cargo workspace with a headless `palimpsest-sim-core` and focused domain crates (`sim-entity`, `sim-time`, `sim-events`, `sim-scheduler`, `sim-storage`), recorded boundary rules in ADR-0001, and confirmed the spike on 2026-08-29. Phase 1 (Master Spec §84) builds a Micro World Kernel from World Grid, Terrain, Local Tile, Person Entity, Basic Movement, Time, Needs, and Basic Utility AI. Before any Phase 1 domain logic was written, the repository needed two new, minimal crate boundaries with a verified inward dependency direction, so that world/terrain logic and AI/behavior logic each got an independently testable, Godot-free, LLM-free home. This historical Task added only those boundaries and audited the existing direction; later CHRON-019..026 supplied the domain logic without changing the boundary contract.

## Objective
Add the minimal `palimpsest-sim-world` and `palimpsest-sim-ai` crate boundaries and the workspace wiring, then audit and preserve the existing inward dependency direction, such that later Phase 1 Tasks (CHRON-019..026) can populate them without re-negotiating module layout.

## Scope
- Add `palimpsest-sim-world` as a headless, Godot-free, LLM-free library crate that will host World/LocalGrid coordinates, Terrain, deterministic world generation, Activity Sites, and local-grid pathfinding (CHRON-019, CHRON-020, CHRON-023, CHRON-024).
- Add `palimpsest-sim-ai` as a headless, Godot-free, LLM-free library crate that will host Needs, Action/Decision-Trace contracts, and Utility scoring/selection (CHRON-022, CHRON-025, CHRON-026).
- Add both crates to the workspace `members` in `Cargo.toml`; keep resolver 3, edition 2024, workspace lint inheritance, and `unsafe_code = "forbid"`.
- Add crate documentation and empty module roots only; do not invent public marker types that later become accidental API.
- Register the dependency direction as allow-sets: `sim-world` may depend on `sim-entity`/`sim-time`/`serde`; `sim-ai` may depend on `sim-world`/`sim-entity`/`sim-time`/`serde`. CHRON-018's empty skeletons were not required to add every permitted edge; later legitimate edges are retained. All simulation/domain crates must not depend outward on the outer `godot-bridge`; the additional forbidden dependencies (`sim-core`, storage, `bevy_ecs`, and Godot/LLM runtimes) apply specifically to the `sim-world`/`sim-ai` allow-sets.
- Audit the existing crate direction against ADR-0001 and ADR-0017.
- Document the Phase 1 crate plan in `docs/ARCHITECTURE.md` without contradicting `MASTER_SPEC.md`.

## Out of Scope
- Any World Grid, LocalGrid, coordinate, Terrain, worldgen, Needs, Pathfinding, Action, Decision-Trace, or Utility implementation (these are CHRON-019..026).
- Person Runtime Model / runtime ECS binding and `bevy_ecs` adoption (CHRON-021; ADR-0005 remains provisional).
- Storage/persistence, SQLite, snapshots, Event Store changes, or history.
- Godot bridge, GDExtension, rendering, input, or Scene Tree.
- LLM, NLG, war, politics, religion, magic, historians, Rule Editor, Web client.
- Relaxing, deleting, or re-writing ADR-0001's boundary rules.

## Dependencies
- CHRON-001 (workspace), CHRON-004 (`EntityId`), CHRON-005 (`SimInstant`/`SimDuration`) complete.
- ADR-0001 (workspace boundaries), ADR-0002 (stable identity), ADR-0003 (simulation time) as the boundary authorities.
- CHRON-014/Phase 0 report confirmed; Phase 1 planning authorized.

## Files Modified / Allowed
- `Cargo.toml` (add two workspace members under `crates/`).
- `Cargo.lock` (resolved by the addition; no direct manual edits).
- `crates/sim-world/**` — **planned new crate** (created by this Task); `Cargo.toml` + a minimal `src/lib.rs`.
- `crates/sim-ai/**` — **planned new crate** (created by this Task); `Cargo.toml` + a minimal `src/lib.rs`.
- `docs/adr/ADR-0017-phase-1-crate-boundaries.md` — change status only if the product owner approves this Task; the decision text is already proposed.
- `docs/ARCHITECTURE.md` — record the Phase 1 crate plan and dependency direction (allowed; must not conflict with `MASTER_SPEC.md`).
- `docs/tasks/CHRON-018.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/PERFORMANCE.md`, or any Phase 0 ADR.

## API Contract
- `palimpsest-sim-world` and `palimpsest-sim-ai` expose no public domain API in this Task; crate documentation is sufficient.
- Dependency-direction contract reviewed at the workspace CI/lint gate and whenever dependencies change, using exact Cargo metadata/tree evidence:
  1. `sim-world` normal dependencies must be a subset of `sim-entity`, `sim-time`, and `serde` (and std); no edge is mandatory.
  2. `sim-ai` may depend on `sim-world`, `sim-entity`, `sim-time`, and `serde`; it must not depend on `sim-core`, `sim-storage`, `sim-events`, `sim-scheduler`, `godot-bridge`, or `bevy_ecs`.
  3. Neither crate may depend on Godot or any LLM/library crate; review uses exact dependency sets, not name-substring inference.
  4. `sim-core` remains the headless composition root; it must be able to depend inward on `sim-world` and `sim-ai` (established fully in CHRON-021).
- These rules are recorded in the new ADR and are a gate for every later Phase 1 Task.

## Tests
- `cargo metadata --no-deps --format-version 1` returns both new crates as workspace members.
- Both new crates compile headlessly (`cargo build -p palimpsest-sim-world -p palimpsest-sim-ai`) with no Godot or LLM dependency in any transitive Cargo feature.
- Dependency-direction review passes: exact normal dependency sets for `sim-world`/`sim-ai` and normal trees are recorded; no simulation/domain crate depends outward on `godot-bridge` or an LLM crate.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` all pass with the two crates present.
- Public-API sanity at CHRON-018 completion: neither skeleton crate exported a placeholder marker or speculative domain type.
- Non-regression: existing Phase 0 crates (`sim-core`, `sim-entity`, `sim-time`, `sim-events`, `sim-scheduler`, `sim-storage`) continue to pass all their existing tests unmodified.

## Benchmark
N/A. This Task introduces no runtime behavior and makes no performance claim. It verifies only that the two new crates build headlessly and that the dependency direction is preserved; any throughput measurement belongs to CHRON-024 (pathfinding) or CHRON-026 (Utility AI).

## Definition of Done
- CHRON-018 established `sim-world` and `sim-ai` as headless, Godot-free, LLM-free workspace crates with no public placeholder types or Phase 1 domain logic at that time; later CHRON-019..026 behavior is retained.
- Both are workspace members; the workspace builds, lints, and tests green with them present, and no Phase 0 crate's tests were changed or weakened.
- The dependency-direction contract is codified and reviewed (inward toward domain primitives; no outward dependency on Godot/LLM/storage).
- ADR-0017 is accepted with this Task or the Task remains blocked.
- `docs/ARCHITECTURE.md` reflects the Phase 1 crate plan without conflicting with `MASTER_SPEC.md`.
- Summary: This Task established only boundaries; it did not implement World Grid, Terrain, Needs, Pathfinding, actions, Utility AI, Person runtime, or any domain behavior. Later tasks populated the approved crates.

## Required Completion Report
Report: the exact change summary; the commands actually run; an explicit N/A for benchmark with reason (no runtime behavior); the resulting dependency graph as verified; the allow-set used for `sim-world`/`sim-ai`; known limitations (historical empty roots; Phase 1 runtime `bevy_ecs` remains in the runtime composition boundary); and any blocker. Do not auto-start the next Task or treat this report as a new user acceptance.

## Conditional review closure — REM-002 (2026-08-30)

The six CHRON-018 review clarifications are recorded in ADR-0017 and the
architecture record. The custom `crates/sim-ai/tests/dependency_direction.rs`
integration audit was removed only after its four pre-removal tests passed and
the equivalent present-state review was run with `cargo metadata` and both
normal `cargo tree` commands. `serde_json` remains because current domain
serialization tests use it; no legitimate dependency was removed. The review
is manual/agent evidence at workspace CI/lint and on dependency changes, not an
automatic future-enforcement mechanism.

At review time, normal direct dependencies were `sim-world = {serde}` and
`sim-ai = {palimpsest-sim-time, palimpsest-sim-world, serde}`. The corresponding
transitive normal tree is recorded in
`docs/reports/CHRON-018_WORKSPACE_BOUNDARIES.md`.
