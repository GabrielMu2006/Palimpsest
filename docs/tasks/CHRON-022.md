# CHRON-022 — Needs Model

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Provide a bounded, integer (fixed-point), NaN-free Needs model for hunger and fatigue that advances strictly from elapsed `SimDuration` and provides deterministic signals for Utility AI (CHRON-026) and the action state machine (CHRON-027).

## Context
Master Spec §13 lists `Needs` as a core Person domain; §14's Utility AI pipeline (Perception → Needs → Available Actions → Utility Calculation → Selection → Execution) and §10's integer-second time model (ADR-0003) mean all Need quantities must be exact, bounded, and derived from integer elapsed time — never floats that could produce NaN, drift, or cross-platform non-determinism. The Master Spec's Chaos Test gate (§76) explicitly forbids NaN and demands bounded values. Phase 1 needs the simplest honest model for the Eat/Sleep/Work loop so that Phase 1's "100 NPC move/eat/sleep/work" DoD (Master Spec §84) is meaningful and later systems can key off bounded, reproducible signals.

## Scope
- Add the domain `Needs` value model to `sim-ai`, with exactly two drives: `Hunger` and `Fatigue`. Add only the narrow `sim-core` ECS component integration needed to attach/query that value on a Person.
- Use integer numeric scaling (fixed-point) with a documented `SCALE` factor; there must be no `f32`/`f64` anywhere in the model. Values are stored/updated as integers and the range is explicitly clamped (no overflow/underflow, no NaN — impossible by construction since it is integer-only).
- Provide `advance(elapsed: SimDuration)` that increases both drives by a deterministic rate per elapsed second, saturating at the maximum. `elapsed` must be non-negative (a `SimDuration` already guarantees this per ADR-0003); if an effectively-zero/negative delta is supplied it must be a documented no-op or an explicit error, never an unbounded mutation.
- Provide consumption/depletion operators used by Phase 1 actions: `eat(amount)` reduces Hunger, `sleep(...)` reduces Fatigue (each saturating at the minimum, with documented clamping and no underflow), each driven by the state machine (CHRON-027) rather than auto-applied here.
- Expose a bounded "urgency"/pressure signal per drive (e.g. normalized to `[0, 1000]` or a documented 0..=MAX integer) that Utility AI can weight, without introducing a personality/style trait here.
- Provide equality/ordering/serde for `Needs` (and the drive value type) as fixed-point integers.

## Out of Scope
- Any other drive or need (social, comfort, bladder, cleanliness, recreation, etc.).
- Personality, Values, Preferences, Goals, Memory, or any trait-based weighting of needs.
- Utility scoring or selection (CHRON-026) and the action state machine (CHRON-027); this Task only models the quantity and its elapsed-driven progression.
- Resource consumption, diet/nutrition, food items, inventory, or production chains.
- Death, starvation, exhaustion consequences, or penalties (as a separate model); document thresholds/limits as data only, not as death rules.
- Anything Godot-facing or LLM.

## Dependencies
- CHRON-021 complete (Person runtime model and `sim-core` ECS integration point).
- CHRON-018 (`sim-world`/`sim-ai` boundaries), CHRON-005 (`SimDuration`; ADR-0003), and CHRON-004 (`EntityId`) complete.

## Files Modified / Allowed
- `crates/sim-ai/**` — **planned new crate**. Creates `src/needs.rs` (or `src/needs/mod.rs`) and re-exports the `Needs`/drive types from `src/lib.rs`.
- `crates/sim-core/**` — only the narrow Person component attachment/query integration; the domain model remains owned by `sim-ai`.
- `Cargo.toml`, `Cargo.lock` if a dependency is required (serde is the only expected one).
- `docs/adr/ADR-0013-person-needs-action-boundaries.md` and `docs/adr/ADR-0017-phase-1-crate-boundaries.md` govern this public boundary; divergence requires a new ADR.
- `docs/tasks/CHRON-022.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `Needs { hunger: NeedValue, fatigue: NeedValue }` with:
  - a documented bounded range `[MIN, MAX]` (e.g. `0..=NEED_MAX` where `NEED_MAX` is a fixed-point integer like `100_000`, and `0` is fully satisfied, `NEED_MAX` is the maximum drive).
  - `NEED_SCALE` constants for integer fixed-point interpretation (documented; e.g. 1,000 units per 1.0 full drive).
  - `advance(elapsed: SimDuration) -> Needs` (consuming or `&mut`) that adds `rate * elapsed_seconds` to each drive and clamps to `[MIN, MAX]`; must stay integer-exact and deterministic.
  - `eat(amount: i64)`, `rest(amount: i64)` that clamp at `MIN` (no underflow) and return/report the consumed amount.
  - `hunger() -> NeedValue`, `fatigue() -> NeedValue`, `is_critical()` (or `hunger_pressure()`/`fatigue_pressure()`) returning a bounded integer signal.
  - Default `Needs::default()` is fully satisfied (both drives at `MIN`).
- Serde: `Needs` and `NeedValue` serialize as bounded fixed-point integers and reject out-of-range values on deserialization.
- Invariants to record:
  1. Integer-only: no float type appears in any public `Needs`/`NeedValue` field, private state, or public method signature; this is enforced by the type contract, focused tests, and review.
  2. Bounded: values never fall outside `[MIN, MAX]`; all increments/decrements saturate (do not overflow or underflow).
  3. Deterministic: equal `(Needs, elapsed)` inputs produce bit-identical outputs; no dependence on wall-clock, thread, or hash-random state.
  4. Time-driven: values change only through an explicit `advance(elapsed)` (or an associated action), never implicitly per-frame/per-tick; the model never applies time on its own.

## Tests
- Bounded default: `Needs::default()` has both drives at `MIN`; `is_critical()` is `false`.
- Advance monotonicity: an increasing `elapsed` never decreases a drive; equal drives at `MIN` after a zero/no-op `advance` stay at `MIN`.
- Clamping/no-underflow: `advance` with a huge `elapsed` saturates at `MAX` (no overflow wrap); `eat`/`rest` with a huge amount clamps at `MIN` (no negative value). Values are always within `[MIN, MAX]` after any sequence of operations.
- Fixed-point exactness: advancing by `N` seconds equals `N` applications of advancing by 1 second (additive, integer-exact); the relationship is reproducible across platforms.
- Determinism: identical `(Needs, elapsed)` inputs yield identical integer outputs across repeated calls.
- Zero elapsed: a `SimDuration::ZERO` advance is a no-op. Negative elapsed is unrepresentable by `SimDuration` and therefore needs no runtime branch.
- Non-linearity guard: advancing via a single large `elapsed` and via many small ones produce identical results (fixed-point commutativity/allocation-free associativity, e.g. within integer bounds), OR a documented rounding rule that is itself integer and deterministic.
- Serde round trip for `Needs`/`NeedValue`, including rejection of out-of-range serialized values.
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- Needs `advance` throughput at 100 and 1,000 persons over a simulated 1-year interval, release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report advances/s, updates per person-year, and peak RSS delta; correctness assertions remain enabled.
- This is a Phase 1 per-person baseline; the 10-year/100-person collective cost is gated at the kernel level (CHRON-028/CHRON-032) and is not self-asserted here.

## Definition of Done
- `Needs` models exactly Hunger and Fatigue as bounded integer fixed-point quantities updated only from elapsed `SimDuration`.
- No float type appears anywhere in the `Needs` model; values are always within `[MIN, MAX]` (no NaN, overflow, or underflow), and the model is integer-exact and deterministic.
- Advance is driven by explicit `elapsed`; eat/rest clamps at `MIN`; a bounded urgency/pressure signal is exposed for Utility AI.
- No Phase 2 trait/personality weighting, no resource/diet, and no death/starvation penalty rules are implemented.
- Needs safety/determinism tests pass; the per-100/1,000-person advance benchmark is reproducible and documented or explicitly N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the advance benchmark result (advances/s, RSS delta) or explicit N/A; the list of covered bound/clamp/determinism/serde test cases; known limitations (e.g., only hunger+fatigue, no float by design, no death/penalty, no personality weighting); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
