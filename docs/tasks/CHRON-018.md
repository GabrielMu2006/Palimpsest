# CHRON-018 — Phase 1 Workspace Boundaries

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Context
Phase 0 established a virtual Cargo workspace with a headless `palimpsest-sim-core` and focused domain crates (`sim-entity`, `sim-time`, `sim-events`, `sim-scheduler`, `sim-storage`), recorded boundary rules in ADR-0001, and confirmed the spike on 2026-08-29. Phase 1 (Master Spec §84) builds a Micro World Kernel from World Grid, Terrain, Local Tile, Person Entity, Basic Movement, Time, Needs, and Basic Utility AI. Before any Phase 1 domain logic is written, the repository needs two new, minimal crate boundaries with a verified inward dependency direction, so that world/terrain logic and AI/behavior logic each get an independently testable, Godot-free, LLM-free home. This Task adds only those boundaries and audits the existing direction; it implements no domain logic.

## Objective
Add the minimal `palimpsest-sim-world` and `palimpsest-sim-ai` crate boundaries and the workspace wiring, then audit and preserve the existing inward dependency direction, such that later Phase 1 Tasks (CHRON-019..026) can populate them without re-negotiating module layout.

## Scope
- Add `palimpsest-sim-world` as a headless, Godot-free, LLM-free library crate that will host World/LocalGrid coordinates, Terrain, deterministic world generation, Activity Sites, and local-grid pathfinding (CHRON-019, CHRON-020, CHRON-023, CHRON-024).
- Add `palimpsest-sim-ai` as a headless, Godot-free, LLM-free library crate that will host Needs, Action/Decision-Trace contracts, and Utility scoring/selection (CHRON-022, CHRON-025, CHRON-026).
- Add both crates to the workspace `members` in `Cargo.toml`; keep resolver 3, edition 2024, workspace lint inheritance, and `unsafe_code = "forbid"`.
- Add crate documentation and empty module roots only; do not invent public marker types that later become accidental API.
- Register the dependency direction: `sim-world` depends only on `sim-entity`/`sim-time`/(serde); `sim-ai` may depend on `sim-world`, `sim-entity`, `sim-time`, (serde); neither may depend on `sim-core`, `sim-storage`, `godot-bridge`, `bevy_ecs`, or any Godot/LLM crate.
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
- Dependency-direction contract enforced by the workspace CD/lint and by an audit test or script that asserts the crate dependency graph:
  1. `sim-world` must depend on nothing except `sim-entity`, `sim-time`, and `serde` (and std).
  2. `sim-ai` may depend on `sim-world`, `sim-entity`, `sim-time`, and `serde`; it must not depend on `sim-core`, `sim-storage`, `sim-events`, `sim-scheduler`, `godot-bridge`, or `bevy_ecs`.
  3. Neither crate may depend on Godot or any LLM/library crate.
  4. `sim-core` remains the headless composition root; it must be able to depend inward on `sim-world` and `sim-ai` (established fully in CHRON-021).
- These rules are recorded in the new ADR and are a gate for every later Phase 1 Task.

## Tests
- `cargo metadata --no-deps --format-version 1` returns both new crates as workspace members.
- Both new crates compile headlessly (`cargo build -p palimpsest-sim-world -p palimpsest-sim-ai`) with no Godot or LLM dependency in any transitive Cargo feature.
- Dependency-direction audit passes: a CI/local check asserts the allowed `sim-world`/`sim-ai` dependency set and that no crate in `crates/` depends on `godot-bridge` or an LLM crate.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` all pass with the two crates present.
- Public-API sanity: neither skeleton crate exports a placeholder marker or speculative domain type.
- Non-regression: existing Phase 0 crates (`sim-core`, `sim-entity`, `sim-time`, `sim-events`, `sim-scheduler`, `sim-storage`) continue to pass all their existing tests unmodified.

## Benchmark
N/A. This Task introduces no runtime behavior and makes no performance claim. It verifies only that the two new crates build headlessly and that the dependency direction is preserved; any throughput measurement belongs to CHRON-024 (pathfinding) or CHRON-026 (Utility AI).

## Definition of Done
- `sim-world` and `sim-ai` exist as headless, Godot-free, LLM-free workspace crates with no public placeholder types and no Phase 1 domain logic.
- Both are workspace members; the workspace builds, lints, and tests green with them present, and no Phase 0 crate's tests were changed or weakened.
- The dependency-direction contract is codified and audited (inward toward domain primitives; no outward dependency on Godot/LLM/storage).
- ADR-0017 is accepted with this Task or the Task remains blocked.
- `docs/ARCHITECTURE.md` reflects the Phase 1 crate plan without conflicting with `MASTER_SPEC.md`.
- Summary: This Task establishes only boundaries; it does not implement World Grid, Terrain, Needs, Pathfinding, actions, Utility AI, Person runtime, or any domain behavior.

## Required Completion Report
Report: the exact change summary; the commands actually run; an explicit N/A for benchmark with reason (no runtime behavior); the resulting dependency graph as verified; the allow-set used for `sim-world`/`sim-ai`; known limitations (empty crate roots; `bevy_ecs` remains confined to `sim-core`); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
