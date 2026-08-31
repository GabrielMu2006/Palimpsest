# CHRON-025 — Action and Decision Trace Contracts

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

> Accepted remediation clarification (2026-08-30): ADR-0019 defines validated
> native/serde candidate construction, complete selection sets versus partial
> traces, and selected-key correspondence. It authorizes fallible
> `ActionCandidate::new` and `DecisionTrace::new` without adding action kinds,
> execution, or durable history. See REM-007 and its completion report.

## Objective
Define and implement the action-candidate and decision-trace *contracts* (types + construction) for Phase 1 Utility AI: the enumerable set of candidate actions and the complete, factor-by-factor trace used by Developer Mode's "Why" (Master Spec §72), but without computing any score or making any selection. Scoring/selection is CHRON-026.

## Context
Master Spec §2.4 requires every important outcome to have a traceable causal factor set (§14 "scoring must be interpretable", §72 "Developer Mode must answer why this NPC did this"), and §14 forbids a bare `random_action()`. Master Spec §82/§107 and Developer Mode (§72) mean the trace must be a first-class, complete, ordered record of every factor and its input, so a later system can show the full weighted calculation (CHRON-026) and the player-side simplified Why. Phase 1 needs the *contract* that makes scoring honest, without yet deciding how factors are weighted or how the winner is chosen. Action candidates are the finite set of decisions open to a Person: Move, Eat, Sleep, Work, and (implicit resting) Idle.

## Scope
- Define the Phase 1 `ActionKind` enum (a stable discriminant) as exactly `Move`, `Eat`, `Sleep`, `Work`, `Idle` — matching the Phase-1 closed loop and Master Spec §84/§15. No combat, socialize, protect, or long-horizon goals.
- Define `ActionCandidate`: one action plus a Phase 1 target `LocalCoord` where applicable. Static Activity Sites are values, not Entities.
- Define the candidate-enumeration contract: a provider that, given a Person's state (Location, Needs, available Activity Sites, reachability), yields an ordered, deduplicated, bounded list of `ActionCandidate`s. Enumerate only actions that are currently available (e.g. no `Eat` when at `Needs` satisfied and no `Work` target within reach-eligible set); do not invent or score.
- Define the bounded trace schema used by CHRON-026: ordered factor inputs, evaluated contributions, candidate totals, selected candidate, and tie-break reason. This Task populates only candidate/factor-input data; CHRON-026 supplies weights, contributions, totals, and selection.
- Provide `trace_for(candidate, context) -> DecisionTrace` that fills every factor this candidate references (e.g. hunger level, fatigue level, distance to target, site availability, work progress). It records only inputs — no final score, no selection.
- Keep all factor types bounded integers; no floats/NaN (consistent with CHRON-022 and the Chaos Test §76). No personality/goals/memory/social values enter the factors.

## Out of Scope
- Factor weighting decisions and any score computation (CHRON-026).
- Selection: picking a winner, tie-breaking, or any notion of "best" candidate.
- The action state machine and execution (CHRON-027); this Task only produces candidate/trace data that machine will consume.
- Personality, Values, Preferences, Goals, Memory, Relations, or Knowledge as factors.
- Movement/pathfinding execution (CHRON-024) and needs dynamics (CHRON-022) — only their current-value inputs are referenced.
- RNG / perturbation of scores (CHRON-026); no randomization here.
- Godot, LLM.

## Dependencies
- CHRON-021 (Person identity/location), CHRON-022 (Needs), CHRON-023 (Activity Sites), and CHRON-024 (reachability/path costs) complete.
- CHRON-026 consumes these types; this Task supplies them.

## Files Modified / Allowed
- `crates/sim-ai/**` — **planned new crate**. Creates `src/action.rs` (or `src/directive.rs`) defining `ActionKind`, `ActionCandidate`, and `src/trace.rs` (or `src/decision.rs`) defining `DecisionTrace`/`TraceFactor`/`FactorId`, plus re-exports in `src/lib.rs`.
- `Cargo.toml`, `Cargo.lock` if a dependency is required (serde is the only expected one).
- `docs/adr/ADR-0014-explainable-utility-decision-contract.md` governs the trace; divergence requires ADR review.
- `docs/tasks/CHRON-025.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- `ActionKind` is an exhaustive `Copy` enum: `Move`, `Eat`, `Sleep`, `Work`, `Idle` (serde as a stable string/integer key).
- `ActionCandidate { kind: ActionKind, target: Option<LocalCoord>, order: u64 }`; `order` is a stable enumeration key, not truth or persistent identity.
- `ActionProvider` (or `candidate_actions(context)`) yields an ordered, deduplicated `Vec<ActionCandidate>`:
  - `Move`: one candidate per reachable goal-of-interest (or a single nearest goal), not every tile.
  - `Eat`/`Sleep`/`Work`: one candidate per available site of the matching kind (or a single best-available site).
  - `Idle`: always present as the do-nothing baseline.
- `FactorId` is a stable, exhaustive token for the Phase 1 factor set (e.g. `Hunger`, `Fatigue`, `DistanceToTarget`, `SiteAvailable`, `WorkProgress`).
- `FactorInput { factor: FactorId, input: i64 }` records bounded raw inputs. `DecisionTrace` additionally defines evaluated contributions/totals/selection fields that remain unpopulated until CHRON-026 performs scoring.
- `inputs_for(candidate, context)` fills every required factor input in deterministic order and never computes a score.
- Invariants to record:
  1. The candidate set is deterministic, deduplicated, and bounded; identical context → identical candidate order.
  2. `ActionKind` is exactly the five Phase 1 kinds; no other action variant exists (combat, social, goals explicitly absent).
  3. Every candidate input trace lists its complete factor-input set in stable order; CHRON-026 must not add hidden inputs.
  4. All factor inputs are bounded integers; weights and contributions are CHRON-026 scope.
  5. No decision is made here: no "best", no score, no tie-break, no RNG.

## Tests
- Candidate determinism: identical context yields an identical, deduplicated, ordered candidate list; repeated calls produce byte-identical serialized candidates.
- Closed-loop presence: when a Person has unmet Hunger and an available Meal site, `Eat` is enumerated with that site target; when no `Work` site is available, `Work` is absent; `Idle` is always present.
- Bounded set: candidate count is bounded by a documented constant for a fixed site set; no unbounded enumeration.
- Trace completeness: for each candidate kind, `trace_for` produces the documented full factor set; every referenced factor present; no missing factor; stable order.
- Integer-only: trace values contain no float field.
- Serde round trip for `ActionKind`, `ActionCandidate`, `FactorInput`, and the bounded trace schema; this does not make traces durable Event Store records.
- Deterministic ordering: `ActionKind`/candidate ordering is stable across platforms (no `HashMap` iteration leakage).
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- Candidate enumeration + `trace_for` construction at 100 and 1,000 persons over a fixed context (single local world with 6–20 Activity Sites), release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report candidates/s and full-traces/s, peak RSS delta; correctness assertions remain enabled.
- This is `bench_utility_ai` (Master Spec §75) data-construction half; the scoring/selection half is measured in CHRON-026. It is not the Phase 1 hard gate.

## Definition of Done
- `ActionKind` is exactly `Move`/`Eat`/`Sleep`/`Work`/`Idle`; `ActionCandidate` + a deterministic provider enumerate the closed-loop candidate set.
- `FactorInput`/`FactorId` and the bounded `DecisionTrace` schema provide a complete ordered contract; this Task populates inputs only.
- No score, no selection, no tie-break, no RNG is produced here; only candidate list and factor trace contracts exist.
- No personality/goals/memory/social/combat/goal factors are introduced.
- Candidate/trace determinism, completeness, and integer-only tests pass; the `bench_utility_ai` enumeration half is documented or explicit N/A.

## Required Completion Report
Report: the exact change summary; the commands actually run; the enumeration/trace benchmark result or explicit N/A; the list of covered candidate-determinism/closed-loop/bounded/completeness/integer-type/serde test cases; known limitations (e.g., only five Phase 1 actions, no weighting/selection, factors are inputs only); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
