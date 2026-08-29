# ADR-0012: Phase 1 World Tile Coordinate Model

- Status: Proposed — awaiting product-owner approval with the first implementing Task
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for spatial/tile-model changes

## Context

The Master Spec describes a WORLD GRID → REGION → LOCAL CHUNK → TILE hierarchy
as a long-term target, but Phase 1 must not pre-build multi-region, chunk, LOD,
or streaming infrastructure. Phase 0 already proved a single 128×128 local tile
map renders at 60 FPS (CHRON-011). Phase 1 needs one concrete, well-typed local
spatial model that carries minimal terrain and walkability without inventing the
full geography system.

## Decision

Phase 1 implements a minimal `WorldGrid` that owns exactly one authoritative
128×128 `LocalGrid`. This satisfies the Phase 1 World Grid / Local Tile boundary
without implementing multiple Regions, streaming, or multi-chunk composition.

- Use a strongly typed, validated integer `LocalCoord` rather than raw
  `u16/i32` values scattered through domain code.
- Store tiles in row-major order: index = `y * width + x`, with `x` in
  `[0, 128)`, `y` in `[0, 128)`.
- Provide bounds-checked accessors that return `Result`/`Option` on
  out-of-range access; callers must not index without a checked lookup.
- The Local grid owns cells that later Tasks populate with minimal terrain and
  walkability values.
- Carry no Region, chunk, LOD, streaming, or multiple-Local addressing in Phase
  1 public APIs. The future hierarchy is a superseding design decision, not a
  set of reserved fields.

## Public Contract

- `palimpsest-sim-world` exposes `WorldGrid`, `LocalGrid<T>`, checked row-major
  indexing, and deterministic row-major iteration.
- A `LocalCoord { x: u16, y: u16 }` value object, validated against the fixed
  128×128 extent, is the only spatial coordinate exchanged across module and
  Godot-render boundaries for Phase 1.
- `TerrainKind` and `Walkability` are introduced by the terrain Task as small
  closed domain values; the render snapshot maps them to presentation values.
- No public API in Phase 1 accepts a Region, World, chunk, or LOD selector.

## Consequences

- Coordinates are type-safe and typo-resistant; out-of-range access is explicit.
- Row-major contiguous storage is cache-friendly for the small map and simple to
  serialize in a future snapshot task.
- Movement, person location, and ActivitySite placement share one unambiguous
  coordinate contract.
- The model is intentionally minimal: expanding to multi-region or chunked
  streaming requires new ADR work and is out of Phase 1 scope.

## Rejected / Deferred Alternatives

- Multi-region or multi-Local World Grid expansion in Phase 1: deferred; the
  required Phase 1 `WorldGrid` is intentionally a single-Local container.
- Chunked streaming or LOD tiles in Phase 1: deferred; the 128×128 map is fully
  resident and renders at target FPS without it.
- Raw integer indexing without a typed coordinate: rejected; it produces
  ordering and bounds defects without compile-time safety.
- Column-major layout: rejected; it is not required by any Phase 1 iterating
  pattern and would diverge from the row-major convention selected here.

## Supersedes / Extends

New decision. Consistent with ADR-0001 (workspace boundaries) and the Phase 0
128×128 tile-renderer validation (ADR-0011 does not govern spatial layout). No
prior ADR is superseded.
