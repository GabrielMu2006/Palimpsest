# CHRON-024 — Deterministic Pathfinding

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Implement a deterministic, headless pathfinder over a single 128×128 `LocalGrid` that produces a stable path or a documented unreachable/budget-limited result.

## Context
Phase 1 needs "Basic Movement" (Master Spec §84) so 100 NPC can move to a Meal/Rest/Work site (CHRON-023) to satisfy Needs (CHRON-022). Pathfinding must be deterministic because the entire simulation is deterministic (ADR-0002/0003/0004) and because Developer Mode (Master Spec §70) and the Chaos Test (§76) require reproducibility and no NaN/hangs. A single LocalGrid A* (or equivalent grid search) with a stable tie-break is sufficient; cross-region pathfinding, dynamic/group avoidance, continuous terrain cost modeling, and multi-chunk pathfinding are explicitly deferred. The result must be safe: no panics on an unreachable or empty world, no unbounded work, and a clear cancellation path.

## Scope
- Add a deterministic grid pathfinder to `sim-world` that operates over `LocalGrid<TerrainKind>`.
- Use A* (or an exact, stable-equivalent best-first search) over 4-directional (or 8-directional if intentionally configured) movement on a single local grid; diagonal movement must be documented and consistent if used (or explicitly disabled) to keep the map-square-cell contract from MASTER SPEC §29.
- Stability: identical `(grid, start, goal)` always returns the identical path; a documented deterministic tie-break (e.g. by `LocalCoord` row-major order on equal `f`/`g`) removes any order/hash dependence; no `HashMap`-iteration or `PartialOrd` instability leaks into the result.
- Handle all terminal outcomes without panic: `Found(path)`, `Unreachable`, and `LimitExceeded` under a documented expansion/path-length budget.
- Provide path-limit and node-budget configuration so unbounded searches are impossible on malformed/empty maps.
- Return a bounded path: a `Vec<LocalCoord>` whose length is `<= max_path_len`, where each step is adjacent on the grid and the first step may include `start`; cost is deterministic and integer.

## Out of Scope
- Cross-region, multi-chunk, or world-level pathfinding; region hierarchies; LOD pathfinding.
- Dynamic obstacle avoidance, flocking, group/crowd pathfinding, or rerouting due to moving agents.
- Continuous/analytic terrain cost, slopes, water-depth walks, or weighted terrain costs beyond a documented uniform/terrain-influenced step.
- Path smoothing, turns, "chunks", bot steering, or animation.
- Path caching/incremental recompute in the kernel; navigation grids; hierarchical heuristics.
- Movement execution/stepping (CHRON-027) and Person physics.
- Godot, LLM.

## Dependencies
- CHRON-019 complete (`LocalCoord` ordering, `LocalGrid`, in-bounds safety) and CHRON-020 complete (terrain walkability).
- CHRON-018 (`sim-world` boundary) complete.

## Files Modified / Allowed
- `crates/sim-world/**` — creates `src/pathfinding.rs` and re-exports from `src/lib.rs`.
- `Cargo.toml`, `Cargo.lock` if a dependency is required (prefer a std-only deterministic search; a small priority queue helper is acceptable without a third-party crate).
- `docs/adr/ADR-0017-phase-1-crate-boundaries.md` governs ownership; a different crate requires ADR review.
- `docs/tasks/CHRON-024.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `Path` wraps an ordered `Vec<LocalCoord>` plus a documented integer `cost`; `len()`, `coords()`, `cost()`.
- `PathError` (or `PathOutcome`) distinguishes `Unreachable` and `LimitExceeded { nodes, budget }`.
- `find_path(grid, start, goal, walkable, config) -> Result<Path, PathError>`:
  - `start`/`goal` must be in-bounds and walkable; an out-of-bounds or non-walkable `goal` is a documented `Unreachable` (or a distinct invalid-input error), never a panic.
  - `config` carries `max_nodes` (expansion budget) and `max_path_len`.
- Invariants to record:
  1. For equal inputs (grid, start, goal, config, cancellation state) the returned path and cost are bit-identical across calls and platforms.
  2. Every returned path is a sequence of adjacent, in-bounds, walkable cells of length `<= max_path_len`; no cell repeats with a greater cost under the search policy.
  3. `Unreachable` is returned when `goal` cannot be reached; `LimitExceeded` is returned when the node budget is consumed.
  4. No out-of-bounds/empty/malformed input ever panics; all are terminal, documented outcomes.

## Tests
- Found-and-valid: on an open walkable grid a short path is returned; every consecutive pair is adjacent; every cell is in-bounds and walkable; length ≤ `max_path_len`.
- Determinism: same inputs give the same path across repeated calls; a tie-heavy open grid returns the same path under the documented tie-break; a run from the start == goal yields the documented zero/one-step path.
- Unreachable: a walled-off or fully-impassable grid returns `Unreachable` (no infinite loop, terminates); an empty grid (no walkable cells) also yields a terminal documented outcome, never a panic.
- Budget: a `max_nodes`/`max_path_len` that is too small yields `LimitExceeded`/a truncated-vs-terminal documented outcome.
- Bounds safety: out-of-bounds `start`/`goal`, and a non-walkable `goal`, return documented errors (no panic).
- Invalid-input no-panic: `start==goal` non-walkable, `start` non-walkable, and a `LocalGrid` full of impassable cells each terminate and are handled.
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- Pathfinding on the fixed 128×128 generated map (CHRON-020/default config), release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Run across a representative set of (start, goal) pairs including reachable pairs, an unreachable pair, and a budget-limited query.
- Report median time per query, max nodes expanded, peak path length, and peak RSS delta; correctness assertions remain enabled.
- This is the Phase 1 `bench_pathfinding` (Master Spec §75) baseline; the 100-Person per-tick path cost is gated at the kernel level (CHRON-028) and not self-asserted here.

## Definition of Done
- A deterministic A*-style pathfinder over a single 128×128 `LocalGrid` returns a stable, valid path or a documented `Unreachable`/`LimitExceeded` result.
- Equal inputs → identical path/cost; tie-break is explicit and order-independent; no panic on any out-of-bounds, empty, or malformed input; no unbounded work.
- Explicit node/path budgets prevent unbounded search.
- No cross-region, dynamic-avoidance, group, smoothing, or non-grid-continuous pathfinding is implemented.
- Pathfinding tests pass; the `bench_pathfinding` baseline is reproducible and documented or explicitly N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the pathfinding benchmark result (time/query, expanded nodes, RSS delta) or explicit N/A; the list of covered found/deterministic/unreachable/cancel/budget/out-of-bounds test cases; known limitations (e.g., single local grid, 4- or 8-connectivity per config, no cross-region/dynamic avoidance); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
