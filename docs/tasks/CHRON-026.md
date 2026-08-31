# CHRON-026 — Utility Scoring and Selection

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

> Accepted remediation clarifications (2026-08-30): ADR-0018 governs the
> default Need/Work weight policy and trajectory tests; ADR-0019 governs
> fallible `PerturbationSpec::new`, validated native/serde inputs, complete
> selection-set keys, and trace correspondence. These supersede the weaker
> original construction rules, not the five-action, integer, headless boundary.

> REM-005 implementation (2026-08-30): ADR-0018's accepted Need/Work policy is
> now applied by the default table: Eat/Sleep `SiteAvailable = 0`, Work
> `SiteAvailable = 2,300`; all other weights and scoring mechanics are
> unchanged. The regression and threshold trajectory evidence is recorded in
> [CHRON-026 Utility Scoring](../reports/CHRON-026_UTILITY_SCORING.md).

## Objective
Implement integer Utility scoring and a stable, deterministic selection over the candidate set from CHRON-025, with an explicit, seed-derived perturbation that may be zero, limited to the Phase 1 actions (Move, Eat, Sleep, Work, Idle). No Personality, Values, Goals, Memory, or Social factors are used.

## Context
Master Spec §14 specifies the Utility AI pipeline (Perception → Needs → Available Actions → Utility Calculation → Action Selection → Execution → Event → Memory) and demands scores be interpretable and attributable (§2.4), forbidding a bare `random_action()` (§14 "randomness is a scoring perturbation, not behavior"). §2.4's example shows a scalar utility from component factors (e.g. +31, +24, +17, −21…) yielding a final score; §31(§2.4) explains randomness is allowed only as a scoring perturbation. Phase 1 must make the closed loop (Move/Eat/Sleep/Work/Idle) choose actions from the candidate list deterministically, with an explicit perturbation whose strength is a documentable constant and may be zero (fully deterministic). Scoring must be integer (no float/NaN) for reproducibility (§76 Chaos Test, §70 Developer Mode). No long-range goal planning, no personality traits, no social/memory weighting, no combat.

## Scope
- Add an integer Utility scorer over the `ActionCandidate` set from CHRON-025 that computes, for each candidate, `score = base_term(candidate, factors) + perturbation(candidate)` in integer arithmetic (documented fixed-point scaling to avoid precision/comparison ambiguity). No floats anywhere.
- Base term uses the bounded, order-stable `TraceFactor` inputs from CHRON-025 with an explicit, documented weight per `FactorId`; the summation and any scaling is integer and checked (`checked_*` / saturating, no silent wrap, no NaN because integer-only).
- Perturbation: a deterministic pseudo-random value derived from a `u64` perturbation seed AND the candidate (so identical candidate in a tie can differ deterministically). Provide a documented `PERTURBATION_RANGE` (e.g. `[-ε, +ε]`) and allow a seed/strength that yields **zero perturbation** (a documented "fully deterministic" mode). The perturbation is additive to the base term, never the sole term.
- Selection: choose the candidate with the highest scalar utility. Ties and equal-utility candidates break by a documented deterministic rule that does **not** depend on: HashMap iteration, insertion order (unless made stable), wall time, or thread scheduling. Use a documented, stable secondary key (e.g. `FactorId`-sum then `ActionKind` then target order) so the winner is reproducible.
- Return, alongside the winner, the full `DecisionTrace` for the winner and, for Developer Mode (§72), the ordered per-candidate scores so the full calculation is auditable (hidden/omitted factors absent per CHRON-025 contract).
- Keep all weights and inputs bounded integers; document the minimum/maximum achievable score range.

## Out of Scope
- The action-candidate enumeration and factor-input collection (CHRON-025).
- Action execution and the state machine (CHRON-027); this Task selects the action, it does not run it.
- Personality, Values, Preferences, Goals, Memory, Relations, Social, or any trait-based weighting.
- Long-horizon GOAP/planning, multi-step plans, or finite-horizon (Master Spec §15 short-term only here).
- Combat actions, travel-between-regions, or any action beyond the five Phase 1 kinds.
- Learning, weight adaptation, or experience-based scoring changes.
- Godot, LLM.

## Dependencies
- CHRON-025 complete (`ActionKind`, `ActionCandidate`, `DecisionTrace`, `TraceFactor`, `FactorId`), CHRON-022 complete (`Needs` bounded inputs), CHRON-023 complete (`SiteKind`/availability inputs), CHRON-024 complete (Move target/path cost input if used).
- CHRON-018/001 (`sim-ai` boundary + workspace).
- ADR-0003 style integer-only, deterministic cross-module contract; record any cross-module public API change in the Phase 1 ADR.

## Files Modified / Allowed
- `crates/sim-ai/**` — **planned new crate**. Creates `src/utility.rs` (scorer + selector) and re-exports from `src/lib.rs`.
- `Cargo.toml`, `Cargo.lock` if a dependency is required (serde only; use an in-crate deterministic PRNG if needed, or a documented perturbation function).
- `docs/adr/ADR-0014-explainable-utility-decision-contract.md`; no new ADR is needed unless implementation deviates.
- `docs/tasks/CHRON-026.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `UtilityScore` is a bounded signed integer type representing a scalar utility; range is documented (e.g. `MIN_SCORE..=MAX_SCORE`, a wide integer range).
- `Scorer` (or `score_candidates(candidates, context, perturbation) -> Vec<(ActionCandidate, UtilityScore, Vec<TraceFactor>)>`): integer base term from weights×inputs, plus additive signed perturbation.
- `Weights` provides a documented default weight per `FactorId` (bounded integer; all non-duplicative). If a weight is 0, the factor is still recorded (complete trace) but contributes 0 — never silently dropped, per CHRON-025.
- `PerturbationSpec { seed: u64, range: PerturbationRange }` with `PerturbationRange::Zero` (a documented constant/mode) and a bounded `[-ε, +ε]` on the same integer scale. `Zero` is a first-class value; the default is documented.
- `select_action(candidates, context, perturbation) -> Selection { candidate: ActionCandidate, score: UtilityScore, trace: DecisionTrace, all_scores: Vec<CandidateScore> }`.
- Stable tie-break: primary = utility score (descending); secondary = a documented deterministic key (e.g. `FactorId`-weighted sum; then `ActionKind` ordinal; then target `LocalCoord`/`EntityId` row-major order). No hash-order dependence.
- Invariants to record:
  1. Integer-only scores/perturbation; no `f32`/`f64`; no NaN (impossible by construction); no silent overflow (checked/saturating).
  2. Deterministic: equal inputs (candidates, weights, context, perturbation seed/range) → identical winner and identical full score list across calls and platforms.
  3. Perturbation is additive; a `Zero` perturbation mode yields a fully deterministic winner with no randomization; non-zero perturbation is bounded and never sole-determinative.
  4. Candidate weighting only ever uses factors explicitly listed in the candidate's `DecisionTrace`; no hidden/omitted factor contributes (or all contribute an explicitly 0 weight, still recorded).
  5. Selection returns exactly one winner from a non-empty candidate set; an empty set returns `DecisionError::EmptyCandidates`. Candidate enumeration guarantees Idle is normally present.
  6. The selection uses only the five Phase 1 `ActionKind`s; no goal/personality/social/memory factor.

## Tests
- Integer-only: `UtilityScore`, weights, inputs, and the perturbation are integer types; compile-level assertion (or a unit test) that no float appears in the scoring path; `checked_*`/saturating prevent overflow (a huge-weight + huge-input instance saturates rather than wraps or yields NaN; and is deterministic).
- Base-term correctness: for a known factor set and weight table, the computed base score equals the documented integer arithmetic; a `FactorId` with weight 0 is present in the trace and contributes 0.
- Perturbation-zero mode: with `PerturbationRange::Zero`, the winner is identical regardless of perturbation seed; the winner matches the pure base-term argmax.
- Non-zero perturbation: for a documented bounded range, two candidates differing only via perturbation resolve per the seeded PRNG, are bounded within `[-ε, +ε]`, and are reproducible (same seed → same perturbation ordering).
- Stable selection/tie-break: a deliberate tie in base score resolves deterministically via the secondary key, and is reproducible across repeated calls and across runs (no HashMap/hardware-order dependence).
- Closed-loop behavior: high hunger selects Eat; high fatigue selects Sleep; when both are low and a Work site is reachable, Work outranks Idle under the documented Phase 1 weights.
- Determinism: identical full context (candidates, weights, context, perturbation seed/range) yields the same winner and identical `all_scores` list; a second independent run is byte-identical.
- Empty candidate set: selection returns `DecisionError::EmptyCandidates` and does not synthesize an untraced action.
- `all_scores` completeness: for each candidate there is an entry; the full trace is present and ordered; winner is the max score (or documented tie-break) and `score(candidate) == base + perturbation`.
- Serde round trip for `UtilityScore`, `PerturbationSpec`/range, and `Selection` (winner + all-score list + trace).
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- `select_action` throughput over a representative context (10+ candidates, full trace) at 100 and 1,000 persons, release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report selections/s, candidates scored/s, peak RSS delta, and (if measured) benchmark time per micro-world with a documented candidate count; correctness assertions remain enabled.
- This is the `bench_utility_ai` (Master Spec §75) scoring/selection half; the 100-Person loop is gated at the kernel (CHRON-028) and is not self-asserted here.

## Definition of Done
- Integer Utility scoring (`base = Σ weight×input`, bounded, checked/saturating) and a deterministic selection over the candidate set with a documented, stable tie-break are implemented and return the winner + full candidate score list + winner trace.
- An explicit perturbation (seed, range) is additive; a `Zero` perturbation mode yields a fully deterministic winner; non-zero perturbation is bounded and reproducible.
- Only the five Phase 1 actions are scored/selected; no Personality, Values, Goals, Memory, or Social factor is involved.
- Scoring is integer-only and NaN-free by construction, no silent overflow, and reproducible across calls and platforms.
- Scoring/selection tests pass; the `bench_utility_ai` scoring half is documented or explicit N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the scoring/selection benchmark result (selections/s, RSS delta) or explicit N/A; the list of covered integer-only/base-term/zero-mode/nonzero-perturbation/tie-break/closed-loop/determinism/empty-set/serde test cases; known limitations (e.g., only five Phase 1 actions, no personality/goals/social, perturbation bounded and may be 0, single-action selection); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
