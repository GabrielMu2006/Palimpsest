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

## SC-006 — CHRON-025: "serde is the only expected dependency" vs mandated `LocalCoord` target

- **Found:** 2026-08-29, implementing CHRON-025.
- **Conflict:** `docs/tasks/CHRON-025.md` **Files Modified / Allowed** says
  "`Cargo.toml`, `Cargo.lock` if a dependency is required (serde is the only
  expected one)", but the same file's **API Contract** mandates
  `ActionCandidate { kind: ActionKind, target: Option<LocalCoord>, order: u64 }`,
  and `LocalCoord` lives in `palimpsest-sim-world`. The mandated contract
  therefore requires a dependency beyond serde.
- **Resolution implemented:** `palimpsest-sim-world` was added to
  `crates/sim-ai/Cargo.toml`. The addition was pre-authorized, so no ADR
  change was needed: ADR-0017's dependency direction already places
  `sim-world` below `sim-ai`, the sim-ai allow-set in
  `crates/sim-ai/tests/dependency_direction.rs` already lists
  `palimpsest-sim-world`, and the crate's own Cargo.toml header comment
  documented that allow-set. serde remains the only external dependency.
- **Spec-text correction:** pending; the parenthetical in CHRON-025's Files
  Modified should read "serde plus already allow-listed workspace crates
  (ADR-0017)". This entry is the correction of record.

## SC-007 — CHRON-026: spec references a `TraceFactor` type CHRON-025 never shipped

- **Found:** 2026-08-29, implementing CHRON-026.
- **Conflict:** `docs/tasks/CHRON-026.md` **Scope** and **API Contract**
  reference a `TraceFactor` type from CHRON-025 ("the bounded, order-stable
  `TraceFactor` inputs from CHRON-025"; `Vec<TraceFactor>` in the scorer
  signature), and **Dependencies** lists `TraceFactor` among the CHRON-025
  deliverables. CHRON-025 actually shipped `FactorInput` (the raw input
  record) and `FactorEvaluation` (input + contribution) in
  `crates/sim-ai/src/trace.rs`; no `TraceFactor` type exists.
- **Resolution implemented:** the implementation uses the shipped names:
  `score_candidates` builds on `factor_inputs_for` → `FactorInput` and
  records `FactorEvaluation` values in each `CandidateTrace`. The spec's
  intent (bounded, order-stable factor inputs feeding the scorer) is
  satisfied exactly; only the type name differed.
- **Spec-text correction:** pending; CHRON-026's `TraceFactor` mentions
  should read `FactorInput`/`FactorEvaluation`. This entry is the correction
  of record.

## SC-008 — CHRON-026: "a documented weight per FactorId" vs needs-driven selection

- **Found:** 2026-08-29, implementing CHRON-026.
- **Conflict:** `docs/tasks/CHRON-026.md` **Scope** and **API Contract**
  require "an explicit, documented weight per `FactorId`" (a single weight
  per factor), while the same file's **Tests** require closed-loop behavior
  where high hunger selects `Eat` and high fatigue selects `Sleep`. The
  CHRON-025 `Hunger`/`Fatigue` inputs are identical for every candidate of a
  person (they read the person's pressures, not anything
  candidate-specific), so a single global weight per `FactorId` makes the
  required tests mathematically unsatisfiable: every candidate would get the
  same needs contribution and the ranking would be needs-independent.
- **Resolution implemented:** `Weights` is keyed per (`ActionKind` ×
  `FactorId`) — one `FactorWeights` set per action kind, e.g. `Eat` weights
  `Hunger` +10 while `Sleep` weights `Fatigue` +10. Every candidate still
  records every factor with an explicit (possibly 0) weight and contribution
  in the trace, satisfying CHRON-026 invariant 4 and the complete-trace
  contract. No ADR change is needed: ADR-0014 mandates integer bounded
  inputs, checked arithmetic, and a stable tie-break, not a weight shape.
- **Spec-text correction:** pending; CHRON-026 should say "a documented
  weight per (`ActionKind` × `FactorId`)". This entry is the correction of
  record.

## SC-009 — CHRON-026: the drafted default SiteAvailable weights invert availability

- **Found:** 2026-08-29, implementing CHRON-026.
- **Conflict:** the implementation design pins handed to the CHRON-026
  implementer specified the default `SiteAvailable` weight as **−10_000**
  for `Eat`/`Sleep` (rationale: "sinks fabricated/unreachable targets") and
  documented the achievable base range as [−11_270, 10_000]. Under the
  pinned scoring formula `contribution = weight × input` and the CHRON-025
  input semantics (`SiteAvailable` = 1 when the target site exists and is
  reachable), a negative availability weight penalizes *available* sites:
  `Eat` would score `10·hunger − 5·distance − 10_000 ≤ −10`, so `Eat` could
  never outrank even the `Move` baseline (0 at the same target) and the
  task's own closed-loop Tests (high hunger → `Eat`, high fatigue → `Sleep`,
  satisfied → `Work` beats `Idle`) would be unsatisfiable. The documented
  range [−11_270, 10_000] is likewise only self-consistent under a
  penalty-on-*unavailability* reading, which the pinned formula cannot
  express.
- **Resolution implemented:** the `Eat`/`Sleep` `SiteAvailable` weights are
  **+10_000** (availability bonus): a real, reachable site gains 10_000, so
  a fabricated or unreachable target (input 0) forgoes the bonus and is sunk
  by comparison, exactly the stated intent. The formula
  (`contribution = weight × input`, saturating) and the trace-honesty
  invariant (contribution always equals weight × recorded input) are
  unchanged. The corrected achievable ranges with the default table are
  base ∈ [−1_270, 20_000] and, with ε ≤ `MAX_EPSILON` (100), total ∈
  [−1_370, 20_100]; both are documented on `UtilityScore` and asserted in
  `default_weights_achievable_range_is_as_documented`. All other default
  weights are exactly as drafted. MASTER_SPEC.md is silent on weight values
  (§14 requires only interpretable scoring), so no Change Proposal was
  required.
- **Spec-text correction:** pending; the design-pin table should read
  +10_000 for `Eat`/`Sleep` `SiteAvailable` and the achievable range should
  be restated as above. This entry is the correction of record.
