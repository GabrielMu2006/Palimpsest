<!-- Authored by Kimi Code (AI coding agent); requested by the product owner after the CHRON-019 completion report. -->

# Specification Conflict Log

This log records internal conflicts and ambiguities discovered inside task
specifications or between a task specification and an ADR during Phase 1
implementation, together with the resolution actually implemented.

Rules:

- This log is **not** a Change Proposal. Conflicts with `MASTER_SPEC.md`
  itself still require a `docs/proposals/CP-XXXX.md` and an implementation
  stop. Entries here cover only conflicts *within* or *between* task specs and
  ADRs where the Master Spec is silent.
- Every entry must state: the conflicting texts (with file locations), the
  resolution implemented, the rationale, and whether a spec-text correction is
  still pending.
- New entries are appended by the task completion report that found them.

---

## SC-001 — CHRON-019: grid accessor parameter type

- **Found:** 2026-08-29, implementing CHRON-019.
- **Conflict:** `docs/tasks/CHRON-019.md` **Scope** says "Expose boundary-safe
  accessors for validated coordinates" (i.e. accessors take `LocalCoord`), but
  the same file's **Tests** require that "get/get_mut/set/swap return the
  documented Err(OutOfBounds) for negative and ≥ 128 coordinates". A
  `LocalCoord` is valid by construction, so typed-only accessors make the
  required tests unwritable and `GridError::OutOfBounds` unconstructible.
- **Resolution implemented:** `LocalGrid` accessors (`get`, `get_mut`,
  `get_index`, `set`, `swap`, `contains`) take **raw `(x: i32, y: i32)`** and
  validate internally, returning `Option`/`Err(GridError::OutOfBounds)`.
  `LocalCoord` remains the typed exchange value: it is produced by
  `coords()`, validated by `new`/`from_index`/serde, and provides the fast
  `index()` path. This satisfies the Tests section exactly and keeps
  ADR-0012's "callers must not index without a checked lookup" rule.
- **Spec-text correction:** pending. The Scope sentence in CHRON-019 should
  read "accessors take raw coordinates and validate them" — left as-is because
  task files record what was specified at approval time; this log entry is the
  correction of record.

## SC-002 — CHRON-019: `LocalCoord` integer width

- **Found:** 2026-08-29, implementing CHRON-019.
- **Conflict:** `docs/adr/ADR-0012-world-tile-coordinate-model.md` Public
  Contract shows `LocalCoord { x: u16, y: u16 }`, while
  `docs/tasks/CHRON-019.md` API Contract requires `new(x: i32, y: i32) ->
  Option<Self>` and "serde as two `i32` integers".
- **Resolution implemented:** fields are stored as `u16` (matching ADR-0012);
  the constructor, accessors, and serde wire form all use `i32` (matching
  CHRON-019). Conversion is lossless widening (`u16` → `i32`) outward and
  validated narrowing (`i32` → `u16` via `u16::try_from` + bounds check)
  inward. Both texts are literally satisfied.
- **Spec-text correction:** none required; ADR-0012's `{ x: u16, y: u16 }`
  reads as storage shorthand and holds as implemented.

## SC-003 — CHRON-020: leftover variant names and `Terrain` vs `TerrainKind`

- **Found:** 2026-08-29, implementing CHRON-020.
- **Conflict:** three naming mismatches inside `docs/tasks/CHRON-020.md`:
  1. Tests mention "`Water`/`Mountains` not walkable; `OpenPlains` walkable",
     but Scope fixes exactly `Ground | Water | Rock` — `Mountains` and
     `OpenPlains` do not exist.
  2. The API Contract writes `LocalGrid<Terrain>` while Scope says "Store
     `TerrainKind` directly in the single `LocalGrid<TerrainKind>`".
  3. The API Contract's signature is `WorldMap::generate(seed, config) ->
     LocalGrid<Terrain>` "or equivalent", with no statement of where the seed
     and config are remembered.
- **Resolution implemented:** exactly `TerrainKind { Ground, Water, Rock }`
  exists; the cell type is `TerrainKind` itself (no separate `Terrain`
  struct). `WorldMap::generate(seed, config) -> WorldMap` — the "(or
  equivalent)" reading — so the map carries its own `seed()`/`config()`
  provenance; the single local map is reachable via `WorldMap::local() ->
  &LocalGrid<TerrainKind>`.
- **Spec-text correction:** pending. CHRON-020's Tests should say
  `Water`/`Rock` and `Ground`; the API Contract should say
  `LocalGrid<TerrainKind>` and `-> WorldMap`. Left as-is because task files
  record what was specified at approval time; this entry is the correction of
  record.

## SC-004 — CHRON-023: `record_work` vs `advance_work` naming; undocumented counter max

- **Found:** 2026-08-29, implementing CHRON-023 (parallel subagent).
- **Conflict:** the API Contract names the checked work update `record_work`,
  while the Tests/Benchmark sections name it `advance_work`. Separately, the
  contract requires a "documented max" for `WorkCounter` without giving a
  value.
- **Resolution implemented:** two layers exist — `WorkCounter::advance_work()`
  (the saturating primitive) and `ActivitySites::record_work(coord)` (the
  checked entry point returning the new count), so both specified names exist
  with consistent semantics. The counter max was fixed at 10,000,000 with the
  derivation documented in rustdoc (~25× headroom over 100 NPC × 10 years on
  one site).
- **Spec-text correction:** pending; pick one name in CHRON-023 and state the
  counter cap. This entry is the correction of record.

## SC-005 — CHRON-024: benchmark needs node counts the API contract forbids

- **Found:** 2026-08-29, implementing CHRON-024 (parallel subagent).
- **Conflict:** the Benchmark section requires reporting "max nodes expanded"
  per query, but the API Contract limits `Path` to `coords + cost` — no
  expansion statistics cross the public API. Two smaller tensions: the
  `max_path_len` cap has no assigned terminal outcome (`LimitExceeded` is
  bound to the node budget only), and requiring "out-of-bounds start/goal →
  documented error" conflicts with taking typed `LocalCoord` endpoints (which
  cannot be out of bounds by construction, cf. SC-001).
- **Resolution implemented:** the public API stays exactly as contracted; the
  bench derives exact expansion counts black-box via deterministic budget
  bisection (documented in the bench module). A `max_path_len` cap hit returns
  `Unreachable` (documented). `find_path` takes raw `(i32, i32)` endpoints so
  out-of-bounds input is representable and testable; returned paths contain
  typed `LocalCoord` values.
- **Spec-text correction:** pending; if kernel-side expansion stats are wanted
  later (CHRON-028), they need a small explicit API addition then. This entry
  is the correction of record.
