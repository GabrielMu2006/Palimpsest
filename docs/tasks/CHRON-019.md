# CHRON-019 — World Coordinates and LocalGrid

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Context
Master Spec §29 defines a square-cell map hierarchy (WORLD GRID → REGION → LOCAL CHUNK → TILE) where MVP Local is a single 128×128 grid. Phase 1 needs a strongly typed integer coordinate representation and a boundary-safe single LocalGrid container that later tasks (Terrain CHRON-020, worldgen, Person location CHRON-021, Pathfinding CHRON-024) can rely on without reimplementing index/access checks. This Task establishes only the coordinate types and the single LocalGrid container; it implements no terrain or generation logic.

## Objective
Provide a minimal `WorldGrid` containing one boundary-safe 128×128 `LocalGrid<T>` plus a strongly typed `LocalCoord`, with checked construction/access and deterministic ordering.

## Scope
- Add `LocalCoord`, a validated integer x/y value in `[0, 128)` × `[0, 128)`.
- Add a minimal `WorldGrid<T>` that owns exactly one `LocalGrid<T>` and exposes no Region, multi-Local, chunk, or LOD selector.
- Provide a single `LocalGrid<T>` container with a fixed 128×128 shape (`LOCAL_GRID_WIDTH = 128`, `LOCAL_GRID_HEIGHT = 128`), backed by a flat `Vec<T>` of exactly `128 * 128` cells, and the invariant that every valid `LocalCoord` maps to exactly one in-range index.
- Expose boundary-safe accessors for validated coordinates and fallible raw-coordinate construction; ordinary invalid input never panics.
- Expose `in_bounds(coord) -> bool`, `iter()` over all cells in deterministic row-major order, `coords()` over every grid cell, and an index ↔ coordinate conversion that is total and invertible for the valid range.
- Represent "no tile" as `Option<LocalCoord>`; do not add an invalid/sentinel coordinate value.
- Keep `LocalGrid` generic and storage-agnostic; no terrain/enum coupling is introduced here.
- Serde: `LocalCoord`, `LocalGrid<T>`, and `WorldGrid<T>` round-trip with exact-size validation, independent of ECS/Godot types.

## Out of Scope
- Terrain type, walkability, generation, seeds, or any world-region/epoch simulation.
- Region (multi-chunk) coordinates, chunk streaming, LOD, or coordinate conversion between World and any future Region level.
- Pathfinding, movement, person location semantics, or map mutation by systems.
- `bevy_ecs` binding or runtime handles.
- Anything Godot-facing or LLM.

## Dependencies
- CHRON-018 complete (the `sim-world` crate boundary is established).

## Files Modified / Allowed
- `crates/sim-world/**` — **planned new crate**. This Task creates/extends `src/lib.rs` plus `src/coord.rs` and `src/grid.rs` (module names may vary; all remain within `sim-world`).
- `Cargo.toml`, `Cargo.lock` only if a new internal dependency is required (prefer none; serde is the only expected dependency).
- `docs/adr/ADR-0012-world-tile-coordinate-model.md` — accept without changing its contract, or stop if implementation needs a different spatial model.
- `docs/tasks/CHRON-019.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` unless a genuine conflict requires a Change Proposal first.

## API Contract
- `LocalCoord` is a strongly typed signed integer coordinate restricted to the LocalGrid bounds, with `new(x: i32, y: i32) -> Option<Self>` (rejects out-of-bounds), `x()`, `y()`, an `index() -> usize` in `[0, 128*128)`, and `from_index(usize) -> Option<Self>`. It must implement `Eq/Ord/Hash` with row-major ordering (y then x), and serde as two `i32` integers.
- `LOCAL_GRID_WIDTH` and `LOCAL_GRID_HEIGHT` are public constants both equal to `128`; `LOCAL_GRID_CELL_COUNT` equals `16384`.
- `LocalGrid<T>`:
  - `filled_with(value: T) -> Self` (for `T: Clone`), `from_cells(Vec<T>) -> Result<Self, GridError>` (rejects a non-16384 length).
  - `len() -> usize` == `16384`; `is_empty() -> bool` == `false` for a well-formed grid.
  - `get(coord) -> Option<&T>`, `get_mut(coord) -> Option<&mut T>`, `get_index(usize) -> Option<&T>`.
  - `set(coord, value) -> Result<(), GridError>` and `swap(coord, value) -> Result<T, GridError>` (fallible; no panic on out-of-bounds).
  - `contains(coord) -> bool`; `iter()` yields `&T` in row-major order; `coords()` yields every `LocalCoord` in row-major order.
  - Serde round-trips to/from a flat fixed-length array of length `16384`.
- `WorldGrid<T>` owns exactly one `LocalGrid<T>` and exposes `local()` / `local_mut()`; it has no selector argument because multiple Locals are out of scope.
- `GridError` distinguishes `OutOfBounds { coord }` and `InvalidCellCount { expected, got }`.

## Tests
- Bounds invariants: exactly 16,384 cells; `coords()` yields 16,384 unique `LocalCoord`s; `len() == 16384`; `is_empty() == false`.
- Index ↔ coordinate: for every valid coordinate `index == from_index(index)` and `coords()` ordering matches `index()`; `from_index` rejects indices ≥ 16384.
- Boundary safety: `get/get_mut/set/swap` return the documented `Err(OutOfBounds)` for negative and ≥ 128 coordinates and never panic; `contains` is correct at all four corners and outside.
- Optional position: `None::<LocalCoord>` represents no tile; no invalid `LocalCoord` can be constructed.
- Determinism/iteration: row-major order is exactly `(0,0),(1,0),…,(127,0),(0,1),…`; two identical grids iterate identically.
- Serde round trip for `LocalCoord`, `LocalGrid<u8>`, and `WorldGrid<u8>` plus a 16,384-cell content hash equality; invalid lengths are rejected.
- Ord/Hash consistency for `LocalCoord` (row-major total order).
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- `LocalGrid` construction from a provided 16,384-cell `Vec`, plus a full 16,384-cell sequential `get` scan, release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report grid build time, full-scan time per pass, and peak incremental RSS; correctness assertions remain enabled.
- This is a container baseline for CHRON-020 (worldgen) and CHRON-024 (pathfinding); it is not the Phase 1 hard gate and must not be interpreted as one.

## Definition of Done
- `WorldGrid<T>`, `LocalCoord` (row-major, strongly typed, in-bounds-only), and a single 128×128 `LocalGrid<T>` exist in `sim-world`.
- All grid access is boundary-safe and fallible (no panics on out-of-bounds); index ↔ coordinate is total and invertible over the valid range.
- The grid is generic, storage-agnostic, serde round-trip compatible, and deterministic in iteration.
- No terrain, worldgen, pathfinding, person, movement, or ECS logic is implemented.
- Public coordinate/LocalGrid contract is recorded in the ADR if consumed by another crate; the container benchmark is reproducible and documented or explicitly N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the container benchmark result (build time, full-scan time, RSS delta) or explicit N/A; the list of covered boundary/index/iteration test cases; known limitations (e.g., single 128×128 only, no Region, no worldgen); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
