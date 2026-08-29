# CHRON-023 — Activity Sites

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

## Objective
Provide static Activity Sites (Meal, Rest, Work) that act as the fixed, walkable affordance locations a Person can path to (CHRON-024) and act at (CHRON-027), so the Phase 1 Eat/Sleep/Work loop is closed. These systems model only the static affordance and a bounded work counter; they implement no inventory, resource, production, storage, or economy.

## Context
Master Spec §30 plans a settlement real-simulation with planning/transport/construction, and §22 makes Profession an emergent property rather than a hardcoded role. Phase 1 does not need any of that. It needs only the minimal closed loop: a Person has a Need (CHRON-022); a Utility AI (CHRON-026) selects an action; the state machine (CHRON-027) executes it. For Eat/Sleep/Work to ever be reachable there must be *somewhere* to eat, sleep, and work — a static, walkable, single-tile-or-area affordance whose location is fixed and whose Work is a bounded counter (not an economy). Activity Sites must be static data, not a production/settlement simulation.

## Scope
- Add a minimal `ActivitySites` model to `sim-world`: a deterministic collection of static value records carrying `LocalCoord`, `SiteKind`, and (for Work only) a bounded observation counter. Sites are not domain Entities in Phase 1.
- Define `SiteKind` as exactly `Meal`, `Rest`, `Work` (the Phase 1 closed-loop affordances) — no combat, trade, production, construction, or storage kinds.
- Guarantee each site's `LocalCoord` is walkable for the generated terrain (CHRON-020); store a `SiteKind` affordance so Pathfinding/utility can target it.
- Add a `WorkSite` bounded counter: a per-site bounded `WorkProgress`/`WorkCounter` (integer, clamped to a documented max, deterministic) that advances only when a Person works there (driven by CHRON-027, not auto-advanced here). Meal/Rest sites need no counter (an optional occupancy is permissible but must be bounded and optional).
- Provide query API: `sites_of(kind)`, `find_nearest(coord, kind) -> Option<LocalCoord>` (deterministic row-major tie-break), `site_at(coord)`, and checked work-counter update by coordinate.
- Serde round trip for the value collection; no runtime or persistent entity handles.

## Out of Scope
- Inventory, resource quantities, currency, prices, markets, taxes, wealth, or any economy.
- Production chains, recipes, storage, logistics, transport, buildings, construction, deconstruction, or settlement simulation.
- Consumption of food bars/items; Work does not produce or consume any item here.
- Deadlines, quality, efficiency, skill-based output, or worker assignment queues.
- Site evolution, construction over time, or generated-from-requirements sites.
- Personality, profession assignment, goals, memory, or social relations.
- Godot, LLM.

## Dependencies
- CHRON-019 complete (`LocalCoord`, `LocalGrid`), CHRON-020 complete (terrain walkability + deterministic worldgen).
- CHRON-026/CHRON-027 will consume the site affordance + bounded counter; this Task only supplies them.

## Files Modified / Allowed
- `crates/sim-world/**` — **planned new crate**. Creates `src/site.rs` (or `src/activity_sites.rs`) and re-exports from `src/lib.rs`.
- `Cargo.toml`, `Cargo.lock` if a dependency is required (serde is the only expected one).
- `docs/adr/ADR-0013-person-needs-action-boundaries.md` governs this value model; divergence requires ADR review.
- `docs/tasks/CHRON-023.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `SiteKind` is an exhaustive `Copy` enum: `Meal`, `Rest`, `Work`.
- `ActivitySite { coord: LocalCoord, kind: SiteKind, work: Option<WorkCounter> }` where:
  - `coord` is guaranteed walkable by the generator contract (or explicitly validated on construction and not constructible otherwise).
  - `work` is `Some` only for `Work` sites and `None` otherwise (type-level/document invariant).
- `WorkCounter` is a saturating `u64` observation count; it has no reset/game-resource semantics in Phase 1.
- `ActivitySites` exposes:
  - `sites_of(kind) -> impl Iterator<Item = &ActivitySite>` in row-major coordinate order.
  - `find_nearest(coord, kind) -> Option<LocalCoord>` with row-major tie-break.
  - `site_at(coord) -> Option<&ActivitySite>` and checked `record_work(coord)`.
- `SiteError` distinguishes `UnknownSite(LocalCoord)` and `NotAWorkSite(LocalCoord)`.
- Invariants to record:
  1. All site coordinates are in-bounds and walkable; there is at least one site of each of `Meal`, `Rest`, `Work` under a default world/placement fixture.
  2. `WorkCounter` is integer, bounded `[0, max]`, and never negative/overflowing.
  3. `ActivitySites` stores no EntityId or runtime ECS handle.
  4. Sites are static: they neither move, spawn, consume, nor produce anything.

## Tests
- Bounded counter: `advance_work` never exceeds `max` (clamps), never goes below `0`, and is integer-exact; a non-`Work` site returns `NotAWorkSite`.
- Type invariant: every `Work` site has `work = Some(...)` and every `Meal`/`Rest` site has `work = None`; construction of a `Work` site without a counter is rejected.
- Walkability: for a generated default terrain grid, every site `coord` is in-bounds and `is_walkable`.
- Presence: for a default placement fixture there is at least one `Meal`, one `Rest`, and one `Work` site.
- Nearest-query determinism: `find_nearest` returns the same `LocalCoord` for a fixed site set and query coordinate; ties resolve row-major; absent kinds return `None`.
- Serde round trip for `SiteKind`, `ActivitySite`, `WorkCounter`, and `ActivitySites`, with no entity handles.
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- `find_nearest` cost across a site set sized for a micro settlement (e.g. ~6–20 sites) and `advance_work` throughput, release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report query/advance time per op and peak RSS delta; correctness assertions enabled.
- This is a small static-data baseline; the per-Person Work-loop cost is realized in CHRON-027/CHRON-028 and is not self-asserted here.

## Definition of Done
- `ActivitySites` provides static, walkable `Meal`/`Rest`/`Work` value records with a saturating integer `WorkCounter` on Work sites.
- All site coordinates are in-bounds and walkable; each kind is represented under a default placement fixture; sites are static and produce/consume nothing.
- `find_nearest` and `advance_work` are deterministic with a documented tie-break and clamp.
- No inventory, resource, production, storage, market, construction, or settlement simulation is implemented.
- The site-affordance tests pass; the site-set benchmark is reproducible and documented or explicitly N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the query/advance benchmark result or explicit N/A; the list of covered counter/type/walkability/presence/nearest/serde test cases; known limitations (e.g., static sites only, bounded work counter, no economy/production); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
