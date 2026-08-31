# ADR-0018: Phase 1 Utility Need/Work Thresholds

- Status: Accepted — product owner confirmed the four follow-up items on 2026-08-30
- Date: 2026-08-30
- Decision owners: Product owner
- Task: REM-004 in `docs/reports/PHASE_1_REVIEW_REMEDIATION_PLAN_V1.md`
- Extends: ADR-0013 and ADR-0014; does not supersede the Master Spec

## Context

The CHRON-026 defaults give reachable Eat/Sleep candidates a 10,000-point
availability bonus. CHRON-022 increases raw hunger/fatigue by 1/2 per simulated
second. After one second both candidates exist even though their integer
pressure is still zero; they then outrank ordinary Work. Static tests at exactly
zero raw needs do not detect this trajectory regression.

The remediation plan's initial weight-only candidate (Eat/Sleep availability
0, Work availability 2,000) is not sufficient for its own reference threshold:
at pressure 200, Eat scores 1,990 while Work scores 1,980. This ADR proposes a
specific corrected table, not an assertion that the initial suggestion passed.

## Scope and Boundaries

Decide default Phase 1 scoring only. Keep the current five action kinds, five
observable factors, candidate provider, Needs rates, integer arithmetic,
explicit seeded perturbation, and stable tie rule. Do not add action execution,
candidate desire-gating, personality, economy, adaptive tuning, or any Phase 2
system. REM-005 implements this decision only after acceptance; CHRON-027 still
requires separate authorization.

## Decision

### Default weights

| Action | Hunger | Fatigue | DistanceToTarget | SiteAvailable | WorkProgress |
|---|---:|---:|---:|---:|---:|
| Move | 0 | 0 | -5 | 10 | 0 |
| Eat | 10 | 0 | -5 | 0 | 0 |
| Sleep | 0 | 10 | -5 | 0 | 0 |
| Work | 0 | 0 | -5 | **2,300** | 0 |
| Idle | 0 | 0 | 0 | -50 | 0 |

All five factors remain visible in every trace, including zero-weight factors.
The table changes defaults, not the configurable `Weights` API. Custom weights
are not promised the default policy's thresholds. Scoring keeps saturating
`i64` operations; no hidden eligibility or emergency-priority branch is added.

The 2,300 Work bonus also leaves a 290-point margin at pressure 200 in the
reference context. Even opposite perturbations at the existing maximum
epsilon 100 cannot erase that margin. This is a small, explicit robustness
margin, not a new random decision mechanism. The Phase 1 default remains zero
perturbation.

### Reproducible reference context

Reuse the existing CHRON-026 fixture, not a new unreviewed golden world:

- `WorldSeed::new(25_025)` and `WorldGenConfig::default()`.
- Origin: first row-major coordinate with a fully walkable 3-by-3 block.
- Person at origin; Meal at origin + (2, 0), Rest at origin + (0, 2), Work at
  origin + (2, 2), WorkCounter 3; default pathfinding budget.
- Manhattan distances are respectively 2, 2, 4. Generate candidates using
  `candidate_actions` against the same immutable context used for scoring.
- Pressure `p` is represented by valid raw need `100 * p`, for `0..=1000`.
- Unless a test explicitly concerns perturbation, use `PerturbationSpec::ZERO`.

Here, available Work scores 2,280, Eat scores `10 * hunger_pressure - 10`,
Sleep scores `10 * fatigue_pressure - 10`, nearest Move scores 0, and Idle
scores -50. Eat/Sleep are omitted at raw need zero by the unchanged provider.

### Observable outcomes

| Reference input | Required outcome with zero perturbation |
|---|---|
| Both pressures 0..=200, including nonzero raw needs with pressure 0 | Work |
| Hunger sweep 0..=228, fatigue 0 | Work |
| Hunger 229, fatigue 0 | Eat: ties Work at 2,280; earlier enumeration key wins |
| Hunger 230..=1000, fatigue 0 | Eat, unique maximum |
| Fatigue sweep 0..=228, hunger 0 | Work |
| Fatigue 229, hunger 0 | Sleep: ties Work at 2,280; earlier enumeration key wins |
| Fatigue 230..=1000, hunger 0 | Sleep, unique maximum |
| Hunger >=700 and strictly greater than fatigue | Eat |
| Fatigue >=700 and strictly greater than hunger | Sleep |
| Equal hunger/fatigue >=229 | Eat; both needs tie and Eat precedes Sleep |
| Fresh Needs advanced by one simulated second | Work: raw 1/2, pressures 0/0 |
| No activity sites | Idle only; selection does not synthesize a target |

These are selection results, not promises that an action has executed or a
need has subsequently decreased. For nonzero perturbation, near ties may
change winner deterministically by seed; the exact 229 crossover and
one-pressure-unit dominance are zero-perturbation guarantees. Both pressures
<=200 still select Work in the reference context for any permitted epsilon,
because the margin exceeds the largest possible pairwise perturbation change.

### Distance and ties

For reachable Work and a reachable need site, the unperturbed equality is
`10 * pressure = 2300 + 5 * (need_distance - work_distance)`.
Integer scores, not a rounded floating-point threshold, decide the winner.
Use lower candidate `order` on exact score ties as in ADR-0014. Input vector
permutation must not replace that stable key as tie precedence.

Distance can move an ordinary crossover. At critical pressure >=900, even
the largest permitted Manhattan distance 254 gives need score >=7,730;
Work's maximum is 2,300. Thus distance and bounded epsilon cannot make
ordinary Work defeat an available critical-need action under these defaults.
Both-high comparisons still use scores, not a new emergency override.

For valid provider output the reachable site's existence is already enforced
by candidate enumeration. This change does not make arbitrary stale or
fabricated candidate inputs executable; structural validation in ADR-0019
does not prove contextual reachability. A future executor must recheck action
preconditions against simulation truth. Do not hide that integration boundary
behind an arbitrary 10,000-point availability bonus.

## Required REM-005 Tests

1. Add the one-second regression using actual `Needs::advance` and
   `SimDuration`, not manually invented scores. It must fail with the old
   default table and pass with the accepted table.
2. Sweep each need's pressure 0..=1000 with the other at zero. Assert exact
   winner ranges, 228/229/230 boundaries, tie reason, scores, and repeatability.
3. Cover both-low pairs (0, 1, 199, 200), raw values 1/2/99 (pressure zero),
   both-high unequal/equal cases, and 699/700/900/1000 boundaries.
4. Retain trace completeness, saturating arithmetic, seeded perturbation,
   zero perturbation, serde, and stable-key tie tests. Add the low-need
   perturbation-margin assertion; do not claim sampled seeds prove all seeds.
5. Cover no sites and distance-sensitive contexts, including a reachable
   long-distance critical need. Use actual in-bounds walkable fixtures and
   provider output; do not pretend an unreachable fabricated target proves
   an executable action.
6. Update documented default score bounds from the old table: a conservative
   base envelope is [-1,270, 10,000], with epsilon <=100 total envelope
   [-1,370, 10,100]. These are bounds, not claimed attainable extrema.
7. Run the existing Utility smoke benchmark. Full post-remediation M5
   measurements remain REM-008, not an ADR-only performance claim.

## Mandatory Future Integration Test (Not Implemented Here)

The separately approved CHRON-027 executor task must run a deterministic
reachable Meal/Rest/Work fixture across repeated decide/move/execute/advance
cycles. Choose a duration sufficient to cross both Need thresholds using the
then-accepted action durations. Assert positive Work completions, successful
Eat and Sleep with the appropriate need reductions, a return to Work after
both pressures fall below 200, and identical outcomes across repeated runs.
Use completion counters/events rather than selected-candidate counts. Record
the exact duration/seed and expectations in that task before implementation;
do not invent action timing here or call a pressure sweep a closed-loop test.

## Alternatives and Consequences

- Keep Work 2,000: rejected for this proposal; fails the explicit pressure-200
  reference case after removing availability dominance.
- Work 2,100: sufficient with zero perturbation, but its 90-point margin at
  pressure 200 can be erased by allowed opposite perturbations. Prefer 2,300.
- Gate Eat/Sleep until a desire threshold: deferred; mixes feasibility with
  preference and removes low-scoring feasible alternatives from explanations.
- Change Needs growth or add action cooldowns: deferred; changes time/action
  policy and masks a scoring defect with a separate mechanism.
- Keep the original table: rejected; a one-second advance already exposes the
  Work-starvation risk even before action execution exists.

This is preliminary micro-world tuning, not final MVP balancing and not proof
of ten-year stability. It keeps all selection reasoning in the same trace.

## Task Completion / Acceptance Gate

- Dependencies: existing CHRON-022/025/026 and accepted ADR-0013/0014.
- Files modified by REM-004: this ADR only. After acceptance, REM-005 may add
  the approved reference to CHRON-026 within its own file boundary.
- Tests for this ADR-only task: source/fixture inspection and exact arithmetic
  review; no simulation implementation changed or test result claimed.
- Benchmark: N/A — decision record only.
- DoD: table, context, outcomes, invalid assumptions, alternatives, and future
  executor boundary are explicit. Accepted by the product owner on 2026-08-30.

Approval record: the product owner confirmed the proposed four follow-up
items, including this 2,300-baseline policy, and clarified that an instruction
to execute a plan accepts its stated decisions and steps without repeated
approvals. REM-005 is authorized; its implementation and verification must
still satisfy this record. CHRON-027 is not part of that authorization.
