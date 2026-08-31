# ADR-0014: Explainable Utility Decision Contract

- Status: Accepted — approved by the product owner with CHRON-025 on 2026-08-29
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for AI decision-contract changes

## Context

Master Spec requires decision scoring to be interpretable and forbids an
unexplainable `random_action()` as the behavior system body (§14, §72). Phase 1
implements Basic Utility AI over a small candidate set (eat, sleep, work,
travel) and must settle on a deterministic, fully traceable selection contract
rather than a black-box or LLM-in-the-loop choice.

## Decision

Utility scoring produces an ordered list of candidate actions with a complete,
auditable score breakdown. Selection is deterministic.

- Candidate actions are a closed, small Phase 1 set derived from Person needs and
  site affordances (per ADR-0013).
- Utility values are integer or fixed-point integers (not bare floating point),
  computed from bounded inputs so ties and ordering are stable.
- A single stable tie-break rule with documented precedence makes equal scores
  resolve identically across runs even if iteration order varies.
- Selection uses no per-decision hidden randomness. Any randomness is an
  explicit, seeded perturbation chosen before scoring, exposed in the trace, and
  may be set to 0 (zero) entirely in Phase 1. This is a perturbation, never the
  selection mechanism.
- `random_action()` as a selectable decision branch is forbidden.

Every decision must be able to answer "why did this NPC do this?" via a
`DecisionTrace`:

- the candidate set considered;
- each candidate's score and each contributing factor (need, site distance,
  affordance availability, etc.);
- the selected candidate and the tie-break reason;
- any seeded perturbation value applied.

## Public Contract

- The Decision module returns a `DecisionTrace` (complete factor breakdown) plus
  the chosen action from a deterministic `decide(...)` call.
- The trace is a bounded, cloneable value; it is surfaced read-only to Developer
  Tools and the Why Inspector, never mutated back into the simulation.
- Decision traces are runtime diagnostic data by default. They are not appended
  to the durable Event Store for every decision; high-level action outcomes may
  emit separate structured events under ADR-0006.
- The utility type is integer/fixed-point with checked arithmetic.
- The perturbation seed is an explicit, configurable input; default 0 in Phase 1.
- No `random_action()`-style entry point exists in the decision API.

## Consequences

- Decisions are reproducible and testable; the 100-NPC/10-year validation can
  assert exact expected actions for fixed seeds.
- Why Inspector and Developer Mode can expose full factor weights as required,
  with no separate "explanation" layer that could diverge from truth.
- Determinism keeps the Chaos Simulation Test (NaN / runaway / infinite-loop
  checks) meaningful.
- Integer scoring is slightly less expressive than floats in theory, but
  deterministic and safe for long runs; a later phase may widen the utility
  type only behind a superseding ADR.

## Rejected / Deferred Alternatives

- Floating-point utility: rejected for determinism/order-stability and because
  repeated arithmetic on long histories invites drift.
- LLM-guarded action selection: rejected; LLM must never decide simulation
  outcome, and Phase 1 Basic Utility AI is deterministic by design.
- Random as a first-class behavior driver: rejected; it violates the explainable
  decision contract.
- Hidden tie-breaks or unstable ordering: rejected; they make traces and 10-year
  runs non-reproducible.
- Full GOAP/planning in Phase 1: deferred to a later phase (Master Spec §15).

## Supersedes / Extends

New decision; extends `MASTER_SPEC.md` §14/§72 and the Phase 1 Basic Utility AI
scope. Consistent with ADR-0001 (workspace boundaries) and doesn't supersede any
prior ADR.
