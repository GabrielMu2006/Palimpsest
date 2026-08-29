# CHRON-020 — Terrain and Deterministic World Generation

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

## Objective
Provide a minimal terrain model and a deterministic world-generation routine in `sim-world`: the same seed always produces the same 128×128 terrain map, and terrain only defines surface type + walkability. No ecology, resources, climate, or long-history world evolution is implemented.

## Context
Master Spec §29 gives the square-cell hierarchy with a single 128×128 Local; §63 guarantees that a Seed deterministically reproduces the initial world (same seed → same or highly consistent map); §64 keeps MVP advanced options minimal. Phase 1 needs a concrete, reproducible map that People (CHRON-021) can stand on and traverse and that Pathfinding (CHRON-024) respects through walkability. Terrain is deliberately minimal here: it is the walkability/surface substrate only. Ecology, resources, climate, species, and any Region-level geography belong to later Phase 3+ and are excluded.

## Scope
- Add exactly three Phase 1 surface variants: `Ground`, `Water`, and `Rock`.
  `Ground` is walkable; `Water` and `Rock` are impassable. Do not add biome,
  ecology, elevation, moisture, or weighted movement semantics.
- Store `TerrainKind` directly in the single `LocalGrid<TerrainKind>`.
- Add a deterministic world generator: `generate(seed: WorldSeed, config: WorldGenConfig) -> LocalGrid<Terrain>` (or a `WorldMap` wrapper) that guarantees:
  - same seed + same config → byte-identical terrain map;
  - the generation algorithm is fully deterministic (no thread/timing/RNG-state drift, no float-derived coordinates that vary by platform);
  - the default config produces a map with at least one guaranteed walkable "spawn/clear" region and at least one impassable feature, so both traversal and blocked-path outcomes are demonstrable.
- Keep `WorldSeed` a strongly typed `u64` newtype distinct from `EntityId`; zero is a valid, reproducible seed.
- Serde round trip for `WorldSeed`, `TerrainKind`, and `LocalGrid<Terrain>`.

## Out of Scope
- Ecology, species, resources, climate/weather, seasons, vegetation growth, or biome simulation.
- World size options, ocean/mountain-heavy advanced parameters, magic strength, disaster rate, starting era, or historical pre-simulation.
- Region/multi-chunk geography, terrain evolution over time, or any long-horizon world simulation.
- Person/animal population placement, settlements, or any gameplay entity on the map.
- Pathfinding, movement, or person location semantics.
- `bevy_ecs`, Godot, LLM.

## Dependencies
- CHRON-019 complete (the single-Local `WorldGrid`, `LocalGrid<T>`, and `LocalCoord` types exist).
- CHRON-018 complete (`sim-world` crate boundary).
- The implementation must choose one documented deterministic integer generator or hash and lock it with golden-seed tests; it must not use randomized `std` hashing.

## Files Modified / Allowed
- `crates/sim-world/**` — **planned new crate**. Creates `src/terrain.rs` (terrain type + `TerrainKind`) and `src/worldgen.rs` (seed, config, generator) plus re-exports in `src/lib.rs`.
- `Cargo.toml`, `Cargo.lock` only if a third-party PRNG/RNG crate is intentionally added (prefer a small in-crate deterministic generator to avoid a new dependency; if a crate is added it must be pinned and its determinism documented).
- `docs/adr/ADR-0012-world-tile-coordinate-model.md` governs the single-Local spatial boundary; divergence requires ADR review before code.
- `docs/tasks/CHRON-020.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `WorldSeed` is a `u64` newtype (serde as a plain integer), never co-typed with `EntityId`; all `u64` values, including zero, are valid.
- `TerrainKind` is exactly `Ground | Water | Rock`; `is_walkable()` is the only movement semantic.
- `WorldGenConfig` contains only a documented generator version and the minimum walkable spawn-area size; no user-facing terrain ratios or climate controls.
- `WorldMap::generate(seed, config) -> LocalGrid<Terrain>` (or equivalent) is the sole public entry point, returning a single 128×128 local map.
- Guarantees to record:
  1. Determinism: for equal `(seed, config)` the full map and its serialized bytes are identical across calls, runs, and platforms.
  2. `LocalGrid<Terrain>` produced has a guaranteed generated walkable spawn region and at least one impassable feature under the default config.
  3. `WorldSeed` never advances or mutates during generation; the allocator/clock are untouched.

## Tests
- Same-seed determinism: `generate(s, cfg)` equals `generate(s, cfg)` cell-for-cell and byte-for-byte (serialize two results and compare); a different seed produces a different map for the given order-statistic/checksum test.
- Different-seed divergence: at least one cell differs for two distinct seeds on a fixed config.
- Zero-seed determinism: seed zero is accepted and reproduces the same golden map hash.
- Walkability consistency: every cell's `is_walkable()` is consistent with the `TerrainKind` invariant (e.g. `Water`/`Mountains` not walkable; `OpenPlains` walkable); no implicit walkability is added by the grid.
- Spawn guarantee: under default config there is a connected walkable spawn region of at least the documented minimum size; there is at least one impassable cell.
- Bounds/size: result grid has exactly 16,384 cells and all coordinates are in range.
- Serde round trip for `WorldSeed`, `TerrainKind`, and `LocalGrid<Terrain>`, including a content-hash equality after round trip.
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- Full-world generation over a fixed documented seed corpus on the M5 16 GB reference machine, release build, ten post-warm-up samples per seed, median reported.
- Report generation wall-time per map, serialized map bytes, and peak incremental RSS delta.
- Correctness assertions remain enabled; treat this as a determinism/preview-cost baseline for later CHRON-024 pathfinding and not as the Phase 1 hard gate.

## Definition of Done
- `TerrainKind`/`Terrain` (minimal surface + walkability) and a deterministic `WorldMap::generate(seed, config)`-style entry point exist in `sim-world`.
- Same seed + config always yields a byte-identical 128×128 map; `WorldSeed` is a `u64` newtype separate from `EntityId`.
- Default config yields at least one guaranteed walkable spawn area and at least one impassable feature.
- No ecology, resources, climate, species, region-level geography, or terrain evolution is implemented.
- Determinism and generation tests pass; the worldgen benchmark is reproducible and documented or explicitly N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the generation benchmark result (per-map time, bytes, RSS delta) or explicit N/A; the list of covered determinism/walkability/spawn/serde test cases; known limitations (e.g., minimal terrain only, no ecology/resources/climate, single map, no region); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
