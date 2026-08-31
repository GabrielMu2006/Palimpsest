# Phase 1 Code Review Remediation Plan V1

- Status: REM-001..008 implementation/evidence complete, including approved REM-008A peak RSS tooling — see §13; REM-009 accepted by the product owner on 2026-08-31 (see §14). CHRON-027 was separately approved with the P1-REMAINING execution plan and completed on 2026-08-30.
- Date: 2026-08-30
- Visibility policy updated: product-owner decision of 2026-08-30 — repository remains public
- Scope: Findings identified on `phase-1-planning` at `e5b0aeb`
- Authority: `MASTER_SPEC.md`, `AGENTS.md`, ADR-0001, ADR-0011, ADR-0014, ADR-0017

## 1. Purpose

This document turns the Phase 1 progress/code-review findings into bounded,
executable remediation Tasks. It does not approve or implement any code change.
`MASTER_SPEC.md` remains read-only, and no item below authorizes Phase 2 content.

The plan preserves these non-negotiable boundaries:

- Rust Simulation Core remains authoritative and fully headless.
- Godot remains a presentation/input/rendering client only.
- Persistent identity is always `EntityId`; a runtime ECS handle is ephemeral.
- Utility AI remains deterministic, integer-only, explainable, and LLM-free.
- Public API and serialization-contract changes require an accepted ADR.
- Existing tests may be corrected or replaced only when the replacement verifies
  the same or a stronger contract; they must not be removed to make checks pass.
- Completion of this remediation does not authorize CHRON-027 or any later Task.

## 2. Reviewed State

At the review point:

- Phase 0 is complete and product-owner confirmed.
- CHRON-018 through CHRON-026 have implementation commits on
  `phase-1-planning`.
- The product owner confirmed that the combined implementation/commit grouping
  for CHRON-019 through CHRON-026 was authorized. This is not a remediation
  blocker.
- CHRON-027 through CHRON-036 remain Proposed and unimplemented.
- The Draft PR CI checks pass, but required Phase 1 M5 benchmark records and
  per-Task completion reports are not yet present.
- Historical review evidence: a private-repository setting caused the branch
  protection API to return HTTP 403. The product owner superseded the earlier
  private requirement on 2026-08-30: the repository must now remain public on an
  ongoing basis. This documentation update does not change or reverify remote
  settings; REM-001 checks live public visibility and `main` protection.

## 3. Finding Disposition

| ID | Finding | Disposition | Blocking CHRON-027? |
|---|---|---|---|
| F-01 | Repository visibility policy and protection evidence need reconciliation | Policy resolved: continuously public by owner decision on 2026-08-30; live visibility/protection verification remains REM-001 | No pending visibility decision; live protection evidence remains required |
| F-02 | Public `PersonRuntime::runtime_handle` leaks `bevy_ecs::Entity` | Fix under existing ADR-0011; no new architecture decision needed | Yes |
| F-03 | Current Utility weights can starve Work after time advances | Add an explicit Phase 1 low-need/work policy in ADR-0018, then tune and test | Yes |
| F-04 | `PerturbationSpec` deserialization bypasses constructor validation | Define validated wire construction in ADR-0019, then implement | Yes |
| F-05 | `ActionCandidate` public construction/Serde permits invalid values and ambiguous order keys | Define candidate invariants in ADR-0019, then implement | Yes |
| F-06 | CHRON-018 conditional review corrections are incomplete | Apply the six accepted corrections without adding domain behavior | Yes |
| F-07 | CHRON-019..026 authorization/commit grouping was not evident in repository records | Resolved by product-owner confirmation; record only, do not rewrite history | No |
| F-08 | Required M5 benchmarks and completion reports for CHRON-019..026 are absent | Run after code remediation and publish reproducible reports | Yes for final acceptance; not a reason to alter architecture |

## 4. Recommended Decisions

### D-01 — Repository remains public

Accepted product-owner decision, 2026-08-30: keep the repository public on an
ongoing basis. This supersedes the prior private-visibility requirement and the
recommendation to upgrade GitHub for private-repository protection. Keep the
exact `main` protections:

- strict required status checks;
- `rust-quality-and-smoke-benchmarks` required;
- `godot-macos-integration` required;
- administrator enforcement enabled;
- force pushes disabled;
- branch deletion disabled.

Verify live visibility and protection before claiming enforcement. If either
differs from this policy, report the mismatch and obtain authorization for the
required remote-setting change. A documentation edit alone is not verification.

Do not switch the repository to private, require a paid-plan upgrade as a
prerequisite under the superseded private policy, weaken CI, or treat a manual
merge convention as equivalent to enforced branch protection.

### D-02 — Runtime ECS handles are crate-internal

Recommended: `bevy_ecs::Entity` may exist only inside `palimpsest-sim-core`.
`PersonRuntime::runtime_handle` becomes crate-private or test-private. Public
callers receive stable domain views keyed by `EntityId` only.

This restores ADR-0011; it does not change the accepted ECS decision and does
not need a new ADR.

### D-03 — Low needs must not suppress Work

Recommended Phase 1 tuning policy:

- keep Eat and Sleep physically available when their corresponding need is
  non-zero;
- keep Work available whenever a reachable Work site exists;
- remove the `10_000` Eat/Sleep availability dominance from default weights;
- use need pressure as the principal reason for Eat/Sleep;
- Work must win in the reference context while hunger and fatigue pressure are
  both at or below 200/1000;
- Eat must win in the reference context at hunger pressure 700/1000 when hunger
  dominates fatigue;
- Sleep must win in the reference context at fatigue pressure 700/1000 when
  fatigue dominates hunger;
- exact threshold crossings must be deterministic and documented;
- distance may break close choices but must not make a critical need lose to
  ordinary Work in the reference micro-world.

Initial implementation candidate: preserve the pressure coefficients and
change Eat/Sleep `SiteAvailable` weights from `10_000` to `0`, leaving Work's
availability baseline at `2_000`. This candidate is not final until ADR-0018 is
accepted and trajectory tests demonstrate the required crossings.

Changing candidate gating instead is deferred. Gating mixes desire with action
feasibility and would make traces less complete; it should be chosen only if
weight-only tuning cannot meet the closed-loop tests.

### D-04 — Serialized domain values must satisfy the same invariants as native construction

Recommended:

- `PerturbationSpec` uses validated deserialization; invalid epsilon is rejected
  rather than silently clamped.
- `ActionCandidate` uses validated construction/deserialization.
- `Idle` must have no target; Move/Eat/Sleep/Work must have a target.
- A candidate collection passed to selection must have unique, contiguous
  enumeration keys `0..n-1` and no duplicate `(kind, target)` pair.
- Invalid candidate collections return a typed error; they do not panic, repair
  themselves silently, or produce an ambiguous `DecisionTrace`.
- The normal `candidate_actions` provider remains deterministic and emits a
  valid collection by construction.

These are public API and serialization decisions, so ADR-0019 must be accepted
before implementation.

## 5. Remediation Task DAG

```text
REM-001 Verify public repository / main protection ─────────────┐
                                                                │
REM-002 Close CHRON-018 conditional review ─────────────────────┤
                                                                │
REM-003 Seal runtime ECS handle API ─────────────────────────────┤
                                                                ├─> REM-008 Verification and completion reports
REM-004 Propose/accept ADR-0018 ─> REM-005 Utility tuning ───────┤
                                                                │
REM-006 Propose/accept ADR-0019 ─> REM-007 Wire invariants ──────┘

REM-008 ─> REM-009 Product-owner remediation acceptance gate
REM-009 ─> CHRON-027 may be considered separately; it is not auto-approved
```

REM-001, REM-002, REM-003, REM-004, and REM-006 have no dependency on one
another. Only one implementation Task may be executed at a time unless the
product owner explicitly authorizes a bounded parallel batch.

## 6. Executable Tasks

### REM-001 — Verify Public Repository and Enforceable `main` Protection

#### Context

The product owner decided on 2026-08-30 that the repository must remain public.
The earlier private-repository upgrade/manual-policy recommendation is
superseded. Current visibility and protection must be verified against D-01;
historical successful checks are not proof of current enforcement.

#### Scope

- Inspect live repository visibility and `main` protection read-only.
- If settings differ from D-01, report the exact mismatch and obtain explicit
  authorization before changing remote visibility or branch protection.
- When authorized, align visibility with `PUBLIC` and restore the exact `main`
  protections listed in D-01; do not treat this document as proof of execution.
- Record the verified settings and check timestamp in the completion evidence.

#### Out of Scope

- Reversing the public-repository decision or switching the repository to private.
- Purchasing or requiring a paid GitHub plan under the superseded private policy.
- Changing CI job behavior or weakening required checks.
- Merging the Draft PR.
- Modifying simulation code.

#### Dependencies

- Public-visibility decision: accepted by the product owner on 2026-08-30.
- Explicit authorization for any remote-setting mutations found necessary.

#### Files Modified / Allowed

- GitHub repository visibility and `main` protection settings only when their
  mutation is explicitly authorized.
- This report and `docs/PHASE_1_PLAN.md` §14, only to record verification evidence
  or a concrete blocker; no policy reversal or simulation-code edit.

#### Tests / Verification

- `gh repo view GabrielMu2006/Palimpsest --json visibility` reports `PUBLIC`.
- GitHub branch-protection query succeeds rather than returning HTTP 403.
- Required contexts exactly match the two approved CI job names.
- Administrator enforcement is true; force pushes and deletion are false.
- A Draft PR remains unable to merge when either required check is absent or
  failing.

#### Benchmark

N/A — remote governance setting only.

#### Definition of Done

- Public visibility and all exact D-01 `main` protections are verified live with
  recorded evidence. Any mismatch remains a reported blocker until corrected or
  explicitly waived by the product owner; no implicit manual-policy fallback.
- No CI requirement is weakened.
- No source file changes are bundled into this Task.

### REM-002 — Close the CHRON-018 Conditional Review

#### Context

The accepted CHRON-018 review required six clarifications. The current Task and
dependency audit still contain the old wording and custom audit mechanism.

#### Scope

1. State that dependency lists are allow-sets, not dependencies an empty crate
   must add.
2. State that `sim-ai -> sim-world` is a permitted future direction, not a
   dependency CHRON-018 had to add.
3. Replace “no crate in `crates/` depends on the bridge” with the actual rule:
   simulation/domain crates do not depend outward on `godot-bridge`; the bridge
   is an outer adapter.
4. Express the LLM boundary using exact permitted dependency sets obtained from
   Cargo metadata. Do not infer architecture from dependency-name substrings.
5. Remove the custom dependency-audit integration infrastructure only after an
   equivalent or stronger review using existing `cargo metadata` and
   `cargo tree` commands is documented. The purpose is contract correction, not
   making a failing test pass.
6. Correct `workspace CD/lint` to `workspace CI/lint`.
- Preserve current legitimate dependencies used by CHRON-019..026; do not
  rewrite manifests to resemble the historical empty skeleton.
- Record the exact current dependency graph in the completion report.

#### Out of Scope

- Changing crate boundaries.
- Adding a new crate, CI service, audit script, LLM integration, or Godot code.
- Removing a legitimate dependency used by implemented Phase 1 behavior.
- Modifying Phase 1 domain behavior.

#### Dependencies

- Product-owner approval of REM-002.
- ADR-0017 remains the boundary authority.

#### Files Modified / Allowed

- `docs/tasks/CHRON-018.md`
- `docs/adr/ADR-0017-phase-1-crate-boundaries.md`
- `docs/ARCHITECTURE.md`
- `crates/sim-ai/tests/dependency_direction.rs`
- `crates/sim-ai/Cargo.toml`
- `crates/sim-world/Cargo.toml`, comments only (scope addition approved 2026-08-30)
- `Cargo.lock`, only if removing an otherwise-unused audit-only dependency
- `docs/reports/CHRON-018_WORKSPACE_BOUNDARIES.md`

No other file is allowed without expanding and re-approving this Task.

#### API Contract

No domain API changes. The dependency contract remains:

- `sim-world` normal dependencies are a subset of its exact ADR allow-set;
- `sim-ai` normal dependencies are a subset of its exact ADR allow-set;
- simulation/domain crates do not depend on `godot-bridge`;
- no core simulation outcome requires an LLM dependency.

#### Tests / Verification

- Inspect `cargo metadata --no-deps --format-version 1`.
- Inspect normal edges with `cargo tree -p palimpsest-sim-world --edges normal`
  and `cargo tree -p palimpsest-sim-ai --edges normal`.
- Confirm every normal dependency is present in the relevant exact allow-set.
- Confirm no production dependency uses name-substring heuristics as a policy.
- Run workspace format, Clippy, tests, and docs gates.
- Review the final diff to prove no domain implementation changed.

#### Benchmark

N/A — documentation and dependency-contract correction only.

#### Definition of Done

- All six conditional-review items are visibly satisfied.
- The actual dependency direction is no weaker than ADR-0017.
- No custom global Godot/LLM name blacklist remains.
- No domain behavior or public domain API changed.
- The completion report includes commands and exact dependency evidence.

### REM-003 — Seal the Runtime ECS Handle API

#### Context

`PersonRuntime::runtime_handle` is public and returns `bevy_ecs::Entity`.
`#[doc(hidden)]` hides documentation only; it does not enforce ADR-0011.

#### Scope

- Make runtime-handle lookup private to `palimpsest-sim-core` or private to the
  `person` module's tests.
- Keep `EntityId` as the only identity exposed by public Person APIs.
- Preserve spawn, `get`, `location`, `needs`, `set_location`, and `set_needs`
  behavior.
- Keep the internal `EntityId -> Entity` map non-serializable and rebuildable.
- Add or retain tests proving distinct runtime handles internally without
  exposing the handle type to external crates.

#### Out of Scope

- Replacing `bevy_ecs`.
- Changing `EntityId` allocation or serialization.
- Adding persistence, snapshots, Godot bridge APIs, actions, or movement.
- Redesigning `PersonRuntime` beyond the visibility boundary.

#### Dependencies

- Product-owner approval of REM-003.
- ADR-0011 and ADR-0002; no new ADR is required if the change only restores
  their existing contract.

#### Files Modified / Allowed

- `crates/sim-core/src/person.rs`
- `crates/sim-core/src/lib.rs`, only if a re-export must be removed
- existing `crates/sim-core` tests directly affected by visibility
- `docs/reports/CHRON-021_PERSON_RUNTIME.md`

#### API Contract

- Public: every person lookup and mutation is keyed by `EntityId`.
- Internal only: `bevy_ecs::Entity` and the runtime mapping.
- No public return type, parameter, public field, bridge value, event, or serde
  value contains `bevy_ecs::Entity`.

#### Tests / Verification

- Unit test: N spawned persons have N unique `EntityId`s and N distinct internal
  runtime handles.
- Public API review: search public signatures for `bevy_ecs::Entity`/`Entity`.
- Existing spawn/location/needs/determinism tests remain unchanged or stronger.
- Workspace format, Clippy, tests, and docs gates pass.
- `cargo doc` does not expose a runtime-handle lookup.

#### Benchmark

- Run the existing Person spawn smoke benchmark to catch gross regression.
- The full ten-sample M5 result is recorded under REM-008.

#### Definition of Done

- No external crate can obtain a runtime ECS handle through the public API.
- All cross-boundary identity remains `EntityId`.
- No Person behavior changes.
- ADR-0011 is satisfied without amendment.

### REM-004 — Decide ADR-0018: Phase 1 Utility Need/Work Policy

#### Context

Current weights grant Eat and Sleep a `10_000` availability contribution as
soon as the corresponding raw need is non-zero. Needs become non-zero after one
simulated second, so the current reference policy can prevent Work indefinitely.

#### Scope

- Create `ADR-0018-phase-1-utility-need-work-thresholds.md` as Proposed.
- Specify the D-03 threshold behavior, reference contexts, distance treatment,
  deterministic tie behavior, and required trajectory/closed-loop tests.
- Include the weight-only recommended solution and candidate-gating alternative.
- Obtain explicit product-owner acceptance before REM-005.

#### Out of Scope

- Editing weights or code.
- Implementing CHRON-027 action execution.
- Economy, profession, personality, goals, memories, social behavior, or LLMs.
- Claiming that Phase 1 tuning is final MVP balancing.

#### Dependencies

- Product-owner approval to draft ADR-0018 is already covered by this requested
  planning document; acceptance of the ADR remains a separate decision.

#### Files Modified / Allowed

- `docs/adr/ADR-0018-phase-1-utility-need-work-thresholds.md`
- `docs/tasks/CHRON-026.md`, only to reference the accepted clarification

#### Tests

Document test vectors in the ADR; no code test runs in the ADR-only Task.

#### Benchmark

N/A — decision record only.

#### Definition of Done

- The ADR states exact observable behavior rather than only subjective tuning
  language.
- Alternatives and their traceability consequences are documented.
- REM-005 remains blocked until the ADR status is Accepted.

### REM-005 — Correct Utility Trajectory and Work Starvation

#### Context

This Task implements accepted ADR-0018 only. It must not invent new behavior
outside that record.

#### Scope

- Change only default Phase 1 weights/constants required by ADR-0018.
- Add deterministic trajectory tests over pressure 0..1000.
- Add a time-advance regression test showing that one second of elapsed time
  does not make Work permanently unreachable.
- Test the low-need, hungry, tired, both-high, no-site, distance, and exact-tie
  cases.
- Preserve full DecisionTrace factor/contribution reporting.
- Define a mandatory CHRON-027 integration test for multi-action execution; do
  not implement it in this Task.

#### Out of Scope

- Executing actions or moving persons.
- Adding actions beyond Move, Eat, Sleep, Work, Idle.
- Personality, goals, memory, social factors, adaptive weights, or random action.
- Changing Needs growth rates unless ADR-0018 explicitly chooses that
  alternative.

#### Dependencies

- REM-004 complete and ADR-0018 Accepted.
- CHRON-022, CHRON-025, CHRON-026 implementations present.

#### Files Modified / Allowed

- `crates/sim-ai/src/utility.rs`
- existing Utility tests/examples under `crates/sim-ai/**`
- `docs/tasks/CHRON-026.md`
- `docs/reports/CHRON-026_UTILITY_SCORING.md`
- `docs/PERFORMANCE.md`, only for measured results under REM-008

#### API Contract

- No new action kind or hidden factor.
- Scores remain checked/saturating integers.
- Identical inputs produce identical scores, winner, tie-break, and trace.
- Low needs permit Work under the accepted reference context.
- Critical hunger/fatigue dominate ordinary Work under the accepted reference
  context.

#### Tests / Verification

- Pressure sweep 0..1000 is deterministic and has documented crossover points.
- At pressure <=200 for both needs, reference Work beats Eat/Sleep/Idle.
- At hunger pressure >=700 with lower fatigue, reference Eat wins.
- At fatigue pressure >=700 with lower hunger, reference Sleep wins.
- One-second Needs advance from zero still permits Work in the reference world.
- Critical needs do not lose to Work solely due to the Work baseline.
- Existing trace completeness, zero perturbation, tie-break, serde, and
  determinism tests pass.
- Workspace format, Clippy, tests, and docs gates pass.

#### Benchmark

- Run the existing Utility selection smoke benchmark after the change.
- Full M5 ten-sample Utility benchmark is recorded under REM-008.

#### Definition of Done

- The starvation regression fails on the old weights and passes on the accepted
  implementation.
- All ADR-0018 observable thresholds are covered by tests.
- No action execution or Phase 2 factor is introduced.
- Selection remains deterministic and fully explainable.

### REM-006 — Decide ADR-0019: Validated Decision Wire Contracts

#### Context

Derived deserialization can currently construct a `PerturbationSpec` outside
its documented range and an `ActionCandidate` that violates target/order
invariants. Silent scorer clamping makes represented configuration differ from
executed configuration.

#### Scope

- Create `ADR-0019-validated-decision-wire-contracts.md` as Proposed.
- Define the D-04 individual-candidate, candidate-collection, perturbation, and
  error contracts.
- Define backward compatibility for currently serialized Phase 1 diagnostic
  values. No durable Phase 1 save format is claimed.
- Obtain explicit product-owner acceptance before REM-007.

#### Out of Scope

- Database schema or Event Store changes.
- Durable snapshots/save migration.
- New action kinds, new AI factors, or action execution.
- Broad serialization framework changes outside `sim-ai`.

#### Dependencies

- ADR-0014 and ADR-0013.

#### Files Modified / Allowed

- `docs/adr/ADR-0019-validated-decision-wire-contracts.md`
- `docs/tasks/CHRON-025.md` and `docs/tasks/CHRON-026.md`, only to reference the
  accepted clarification

#### Tests

Document valid and invalid JSON vectors in the ADR; no code test runs in the
ADR-only Task.

#### Benchmark

N/A — decision record only.

#### Definition of Done

- Every wire-visible invariant and invalid-input outcome is explicit.
- No silent clamp/repair behavior remains in the accepted design.
- REM-007 remains blocked until the ADR status is Accepted.

### REM-007 — Enforce Candidate and Perturbation Invariants

#### Context

This Task implements accepted ADR-0019 only.

#### Scope

- Route `PerturbationSpec` deserialization through the same range validation as
  native construction.
- Reject epsilon outside `0..=MAX_EPSILON`; remove silent execution-time clamp
  if the accepted ADR makes invalid state unrepresentable.
- Make public candidate construction validated or restrict unchecked
  construction to the crate.
- Validate selection input for target rules, duplicate/non-contiguous order
  keys, and duplicate `(kind, target)` pairs.
- Return typed errors for invalid values/sets.
- Preserve serde keys for all valid existing diagnostic values unless ADR-0019
  explicitly approves a versioned change.

#### Out of Scope

- Changing scoring weights or tie precedence.
- Adding a general-purpose validation framework.
- Persistent save migration or Event Store changes.
- Action execution.

#### Dependencies

- REM-006 complete and ADR-0019 Accepted.

#### Files Modified / Allowed

- `crates/sim-ai/src/action.rs`
- `crates/sim-ai/src/utility.rs`
- `crates/sim-ai/src/trace.rs`, only if the typed error or validated selected key
  requires it
- `crates/sim-ai/src/lib.rs`, only for intentional public re-exports
- existing `crates/sim-ai` tests/examples directly affected
- `docs/reports/CHRON-025_ACTION_TRACE.md`
- `docs/reports/CHRON-026_UTILITY_SCORING.md`

#### API Contract

- Valid native and deserialized values obey identical invariants.
- Invalid input returns an explicit error and does not panic or silently mutate.
- `DecisionTrace.selected` refers to exactly one candidate.
- Valid existing candidate enumeration and scoring remain deterministic.

#### Tests / Verification

- Serde rejects negative epsilon and epsilon above `MAX_EPSILON`.
- Serde accepts boundary epsilon 0 and `MAX_EPSILON`.
- Invalid Idle/target and targetless non-Idle candidates are rejected.
- Duplicate, missing, and non-contiguous order keys are rejected per ADR-0019.
- Duplicate `(kind, target)` candidates are rejected.
- Valid provider output round-trips and selects identically before/after serde.
- Fuzz-like bounded matrix: all five kinds × target present/absent × boundary
  order cases, without adding a new fuzzing dependency.
- Workspace format, Clippy, tests, and docs gates pass.

#### Benchmark

- Compare candidate validation + selection smoke throughput with the current
  baseline; report the delta.
- Full M5 result is recorded under REM-008.

#### Definition of Done

- Invalid decision values cannot enter through public native or serde paths.
- No represented perturbation differs from the perturbation actually executed.
- Traces cannot identify multiple candidates with the same selected key.
- Valid Phase 1 behavior remains deterministic and LLM-free.

### REM-008 — Verification, M5 Benchmarks, and Completion Reports

#### Context

CHRON-019 through CHRON-026 contain benchmark and Required Completion Report
requirements, but the repository currently contains benchmark executables rather
than recorded M5 results/reports.

#### Scope

- Run all required correctness gates after REM-002/003/005/007.
- Run each CHRON-019..026 benchmark on the M5 16 GB reference machine in release
  mode, with the task-specified warm-up/sample method.
- Record exact commands, commit, toolchain, OS/hardware, sample count, median,
  RSS method, raw result location, limitations, and blockers.
- Create one completion report per Task; cross-link results from
  `docs/PERFORMANCE.md` without duplicating contradictory numbers.
- Explicitly label any unavailable measurement N/A with a reason; N/A does not
  satisfy a Task whose DoD requires an actual performance result unless the
  product owner grants a written exception.

#### Out of Scope

- Retuning performance budgets.
- Optimizing code solely because a result looks undesirable; optimization needs
  a separate diagnosed Task.
- Running CHRON-027+, Godot micro-world, 100-NPC/10-year, or Phase 1 final report
  validations before their Tasks are approved and implemented.

#### Dependencies

- REM-002, REM-003, REM-005, and REM-007 complete.
- M5 16 GB reference machine available and otherwise idle enough for repeatable
  samples.

#### Files Modified / Allowed

- `docs/reports/CHRON-018_WORKSPACE_BOUNDARIES.md`
- `docs/reports/CHRON-019_LOCAL_GRID.md`
- `docs/reports/CHRON-020_WORLDGEN.md`
- `docs/reports/CHRON-021_PERSON_RUNTIME.md`
- `docs/reports/CHRON-022_NEEDS.md`
- `docs/reports/CHRON-023_ACTIVITY_SITES.md`
- `docs/reports/CHRON-024_PATHFINDING.md`
- `docs/reports/CHRON-025_ACTION_TRACE.md`
- `docs/reports/CHRON-026_UTILITY_SCORING.md`
- `docs/PERFORMANCE.md`
- Raw benchmark artifacts under a dedicated ignored or documented report-data
  directory only if the existing project convention supports them.

No production code change is allowed in this measurement Task.

#### Tests / Verification

- `cargo fmt --all -- --check`
- workspace Clippy with warnings denied
- workspace tests for all targets/features
- workspace documentation build
- existing Godot macOS integration check
- dependency-direction verification from REM-002
- clean `git status` after generated build artifacts are excluded normally

#### Benchmark

- CHRON-019 LocalGrid construction/full scan.
- CHRON-020 deterministic full-world generation.
- CHRON-021 Person spawn/attach at 100 and 1,000 persons.
- CHRON-022 Needs advance at 100 and 1,000 persons.
- CHRON-023 site lookup/work-counter throughput.
- CHRON-024 fixed-map pathfinding.
- CHRON-025 candidate enumeration/trace construction.
- CHRON-026 Utility selection/scoring.
- Ten post-warm-up samples and median wherever the source Task requires them.

#### Definition of Done

- Every required command and measurement is reproducible from the report.
- Every CHRON-019..026 report contains change summary, tests, benchmark, known
  limitations, and blockers.
- `docs/PERFORMANCE.md` contains one authoritative result per measurement.
- No performance budget was silently relaxed.
- All correctness gates pass at the remediated commit.

### REM-009 — Product-Owner Remediation Acceptance Gate

#### Context

Passing CI is necessary but does not accept architectural, behavioral, or
governance changes.

#### Scope

- Review the REM-001..008 completion evidence.
- Confirm ADR-0018 and ADR-0019 status and implementation conformance.
- Confirm F-01..F-08 dispositions are accurate.
- Product owner explicitly accepts or rejects the remediation batch.
- If accepted, decide separately whether CHRON-027 may begin.

#### Out of Scope

- Implementing CHRON-027.
- Merging unrelated documentation/introduction changes.
- Entering Phase 2.

#### Dependencies

- REM-001 through REM-008 resolved or explicitly waived in writing.

#### Files Modified / Allowed

- This report, only to record final disposition.
- Relevant Task/ADR status lines, only after explicit product-owner acceptance.

#### Tests

Evidence review only; no new code test.

#### Benchmark

N/A — acceptance gate.

#### Definition of Done

- Every finding is Closed, Accepted Risk, or Blocked with an owner.
- The Draft PR scope is understood and no unresolved critical finding is hidden.
- CHRON-027 remains unstarted unless separately approved by the product owner.

## 7. Execution Rules

For every remediation implementation Task:

1. Obtain explicit product-owner approval for that Task.
2. Re-read required authority documents and the relevant accepted ADR.
3. Confirm the worktree is clean or isolate unrelated user changes.
4. Change only the Task's allowed files.
5. Run the Task's tests; do not remove or weaken tests to obtain green results.
6. Record commands actually run and measured results.
7. Stop after the Task completion report; do not auto-start the next Task.

Parallel execution is allowed only when the product owner explicitly authorizes
it and file sets do not overlap. Under this DAG:

- REM-001 can run independently because it is a remote setting Task.
- REM-003 can run independently of the documentation-only REM-004/REM-006.
- REM-005 and REM-007 must not run in parallel because both touch
  `crates/sim-ai/src/utility.rs`.
- REM-002 and REM-007 both affect `sim-ai` task/dependency material and should be
  sequenced to avoid conflicting edits.
- REM-008 runs only after all code fixes, never in parallel with implementation.

## 8. Product-Owner Actions Required

Historical pre-execution checklist: the latest whole-plan approval in §11
satisfies the implementation/ADR/dispatch approvals below. It is not a request
to repeat them. Final evidence and the remaining measurement gap are in §12;
an unmeasured result is not made successful by blanket execution approval.

Before the entire remediation can close, the product owner must:

1. Approve REM-001 verification and any necessary remote-setting correction.
   The continuously public visibility decision is already accepted; no
   private-repository upgrade decision remains pending.
2. Approve REM-002 and REM-003 before their file/code changes.
3. Review and accept/reject proposed ADR-0018 before Utility tuning.
4. Review and accept/reject proposed ADR-0019 before public/serde API changes.
5. Approve REM-008 benchmark execution and report generation.
6. Accept the completed remediation before considering CHRON-027 separately.

## 9. Explicit Non-Goals

This remediation plan does not implement or authorize:

- CHRON-027 action execution or any later Phase 1 Task;
- full NPC AI, long-term planning, personality, memory, relations, or economy;
- war, politics, religion, magic, historians, NLG, LLM, Rule Editor, or web;
- new persistence schemas, Event Store changes, or save migration;
- replacing Godot or `bevy_ecs`;
- Phase 2 work;
- modification of `MASTER_SPEC.md`.

## 10. Historical Checkpoint — Before the Four-Item Confirmation, 2026-08-30

The product owner requested execution via `codex-luna-dispatch`. The parent
started with the bounded REM-003 correction to an already accepted contract,
and prepared ADR drafts locally. This is a partial checkpoint, not blanket
ADR acceptance, final remediation acceptance, or authorization for CHRON-027.
Existing uncommitted public-policy documentation edits were preserved. No
commit, push, merge, or remote-setting write was performed.

### Dispatch Fitness / Ownership

| Task | Decision before dispatch | Evidence and allowed write ownership | Acceptance |
|---|---|---|---|
| REM-003 | READY; one Luna worker | Root cause is the public accessor in `person.rs`; no outside caller found; accepted ADR-0011 supplies the contract. Worker owns `person.rs` and the CHRON-021 report only. | Parent diff review, original tests, positive/negative doctests, full Rust gates and spawn smoke; independently verified |
| REM-001 | KEEP_LOCAL; read-only | Network retry verified PRIVATE and protection HTTP 403. Remote mutations require the explicitly requested authorization. | Blocked on remote correction authority, then live verification |
| REM-002 | PREPARE; not dispatched | Four old audit tests pass. Metadata/tree review confirms exact legitimate dependency sets. Removal would leave a false enforcement comment in `sim-world/Cargo.toml`, which the task whitelist omitted. | Requested a comment-only whitelist extension; no audit test removed or weakened |
| REM-004 | KEEP_LOCAL; ADR design | New behavior requires parent design and owner acceptance; no runtime code written. | ADR-0018 draft complete, acceptance pending |
| REM-006 | KEEP_LOCAL; ADR design | Public/native/serde/partial-trace boundaries require parent design. | ADR-0019 draft complete, acceptance pending |
| REM-005 / REM-007 | BLOCKED | Their required accepted ADRs do not yet exist. Both would write `utility.rs`; never dispatch concurrently. | Not implemented |
| REM-008 / REM-009 | BLOCKED by DAG prerequisites | Remaining code fixes and final measurements/acceptance are not complete. | Not claimed complete |

Only one implementation worker was used. Parent ADR design/read-only checks
ran alongside that bounded implementation; there were no overlapping writes.
Agent: `/root/rem003_runtime_handle`; requested `gpt-5.6-luna`, medium reasoning,
no history fork. The tool confirmed the agent ID, not backend model routing.
No OpenCode, external model API, or new sidebar task was used.

### Completed Code / Independent Evidence

REM-003 closes the accidental public ECS-handle accessor with a private,
test-only helper. All existing core tests remain; a positive stable-ID lookup
doctest and a compile-fail external-access doctest now guard the boundary.
See [CHRON-021 REM-003 report](CHRON-021_PERSON_RUNTIME.md) for exact diff scope,
commands, hardware, before/after smoke output, and measurement limitations.

Parent verification passed: core tests (12), core doctests (2), core Clippy,
`./tools/ci-rust.sh` (including all 151 workspace unit/integration tests,
MSRV 1.95 and seven existing smoke commands), workspace doctests, warning-free
workspace docs, and `git diff --check`. Nothing was skipped or removed to
obtain green results. Master Spec SHA-256 still matches the read-only baseline.
Godot rendered validation and the complete CHRON-019..026 M5 results remain
REM-008; this checkpoint must not stand in for them.

### REM-002 Preparation Evidence (Not Completion)

The parent independently ran:

- `cargo test -p palimpsest-sim-ai --test dependency_direction` — all four pass
  before any proposed removal; this is not a failing-test workaround.
- `cargo metadata --no-deps --format-version 1 --offline` — both Phase 1
  crates are workspace members.
- `cargo tree -p palimpsest-sim-world --edges normal --offline`
- `cargo tree -p palimpsest-sim-ai --edges normal --offline`

Exact current direct normal dependencies:

- `palimpsest-sim-world`: `{serde}`; subset of the allowed
  `{palimpsest-sim-entity, palimpsest-sim-time, serde}`.
- `palimpsest-sim-ai`: `{palimpsest-sim-time, palimpsest-sim-world, serde}`;
  subset of the allowed `{palimpsest-sim-entity, palimpsest-sim-time,
  palimpsest-sim-world, serde}`.
- Transitive normal packages below those two crates are sim-time/sim-world
  as applicable and serde 1.0.229, serde_core 1.0.229, serde_derive 1.0.229,
  proc-macro2 1.0.107, quote 1.0.47, syn 3.0.4, unicode-ident 1.0.24.
  No simulation package in metadata has a direct normal bridge dependency;
  only the bridge has the Godot dependency.
- `serde_json` remains a legitimate dev-dependency used by current domain
  serde tests. It is not an audit-only dependency to delete.

This is present-state review evidence, not a claim that manual review provides
automatic future CI enforcement. REM-002 must explicitly document the change
of audit mechanism if approved; it must not mislabel a removed test as running.
The old audit file and both manifests are still unchanged at this checkpoint.

### Decisions Still Needed

1. REM-001: authorize changing the actual repository from PRIVATE to PUBLIC
   and aligning/verifying the exact approved `main` protections. The live API
   returned PRIVATE and HTTP 403; no protection is currently claimed verified.
2. REM-002: approve the narrow whitelist addition of
   `crates/sim-world/Cargo.toml` **comments only**, to remove the stale reference
   to the audit test when completing the six review corrections. No dependency
   change is proposed.
3. Accept/reject [ADR-0018](../adr/ADR-0018-phase-1-utility-need-work-thresholds.md)
   before REM-005: weight-only correction, Work baseline 2,300, reference
   low-need Work guarantee through pressure 200, zero-perturbation crossover
   229, unchanged Needs growth and candidate feasibility.
4. Accept/reject [ADR-0019](../adr/ADR-0019-validated-decision-wire-contracts.md)
   before REM-007: validated native/serde construction, typed errors,
   contiguous unique selection keys without banning vector permutation,
   preserved partial diagnostic traces, and selected-key correspondence.

ADR statuses are Proposed. REM-005/007 code is untouched. After the necessary
confirmations, execute each next approved task serially, reread the current
interfaces before dispatch, and independently verify it before moving on.

## 11. Approval and Execution Continuation — 2026-08-30

The product owner confirmed all four follow-up items and clarified that an
explicit instruction to execute the identified plan accepts its stated
decisions and implementation steps as a whole. Routine per-Task/dispatch
reconfirmation requirements elsewhere in this document are therefore
satisfied for this remediation plan, not additional gates. This supersedes
the pending approvals in the historical checkpoint above.

- REM-001: the actual change to PUBLIC and restoration/verification of the
  exact D-01 `main` protections are authorized.
- REM-002: the comment-only `sim-world/Cargo.toml` whitelist addition is
  approved; the six planned review corrections may proceed.
- ADR-0018 and ADR-0019 are Accepted; REM-005 and REM-007 may proceed in
  sequence, with separate bounded worker ownership and parent verification.
- REM-008 verification and report generation are included in execution of
  this remediation plan, after its code prerequisites pass.
- No commit, push, PR merge, CHRON-027 implementation, or Phase 2 work is
  added. Completion will report sensitive changes and known limitations;
  approval to implement is not evidence that unrun tests or future outcomes
  have passed. The final evidence review will not be represented as an
  additional user confirmation already obtained.

The updated approval semantics are recorded in `AGENTS.md`. Parent fitness
assessment and independent review remain mandatory, but are not user-facing
approval requests for already settled decisions.

### REM-001 Completed — Live Verification

Verified 2026-08-30, approximately 12:54 +08:00. Parent-owned remote work;
Benchmark N/A. Code and CI workflows unchanged.

Commands actually run (no commit/push/merge):

```sh
gh repo view GabrielMu2006/Palimpsest --json nameWithOwner,visibility,url,defaultBranchRef
gh repo edit GabrielMu2006/Palimpsest --visibility public --accept-visibility-change-consequences
gh api repos/GabrielMu2006/Palimpsest/branches/main/protection
gh api --method PUT repos/GabrielMu2006/Palimpsest/branches/main/protection \
  -F 'required_status_checks[strict]=true' \
  -f 'required_status_checks[contexts][]=rust-quality-and-smoke-benchmarks' \
  -f 'required_status_checks[contexts][]=godot-macos-integration' \
  -F enforce_admins=true -F required_pull_request_reviews=null \
  -F restrictions=null -F allow_force_pushes=false -F allow_deletions=false
gh repo view GabrielMu2006/Palimpsest --json nameWithOwner,visibility,url
gh api repos/GabrielMu2006/Palimpsest/branches/main/protection
gh pr list --repo GabrielMu2006/Palimpsest --state open --json number,url,isDraft,baseRefName,headRefName,statusCheckRollup
gh pr view 1 --repo GabrielMu2006/Palimpsest --json isDraft,mergeStateStatus,mergeable,url,headRefOid
```

The two final protection queries were filtered with `--jq` for the listed
fields. Initial visibility was PRIVATE; after the visibility mutation it was
PUBLIC and protection returned 404, confirming protection needed creation.
The write succeeded and the independent read-back returned:

```json
{"allow_deletions":false,"allow_force_pushes":false,"checks":[{"app_id":15368,"context":"rust-quality-and-smoke-benchmarks"},{"app_id":15368,"context":"godot-macos-integration"}],"contexts":["rust-quality-and-smoke-benchmarks","godot-macos-integration"],"enforce_admins":true,"strict":true}
```

PR [#1](https://github.com/GabrielMu2006/Palimpsest/pull/1) remains Draft.
Its Git mergeability fields are CLEAN/MERGEABLE, which do not remove draft or
required-check restrictions. Existing remote CI is successful on `e5b0aeb`;
it does not certify the unpushed remediation. No deliberate failing check,
absent-check test PR, or merge attempt was created. Negative-path enforcement
is verified by the configured strict/admin-required check policy, not a
destructive remote experiment.

Sensitive change: the repository and accessible history/Actions information
are now public. The source license was not changed. All approved D-01
protections are enabled; this report is point-in-time evidence, not monitoring.

### REM-002 Completed — Independently Accepted

Worker `/root/rem002_boundaries` was requested with `gpt-5.6-luna`; the
dispatcher did not expose backend model routing, so no stronger routing claim
is made. The six review corrections, comment-only manifests and approved
custom-audit removal were accepted after one bounded rework and parent diff,
metadata/tree, formatting and workspace test verification (147/147 at this
checkpoint). See [CHRON-018 evidence](CHRON-018_WORKSPACE_BOUNDARIES.md).

Sensitive change: four passing custom audit tests were removed as the approved
mechanism change, not to bypass a failing test. Existing domain tests and all
dependencies remain. Metadata/tree review is required for future dependency
changes; it is not automatic future CI enforcement. Benchmark N/A.

### REM-005 Completed; REM-007 Ownership

REM-005 was requested from Luna and independently reviewed. One bounded
rework still missed pressure/raw-value and far-critical-need coverage; the
parent took over those exact tests, without further worker retries. The
accepted table is unchanged from ADR-0018. The parent observed the fresh
one-second regression fail with the old weights, restored the accepted
weights, and passed sim-ai 51/51, workspace 154/154, 2 doctests, fmt, Clippy,
rustdoc and diff checks. The sequential prebuilt release baseline is in
[CHRON-026 evidence](CHRON-026_UTILITY_SCORING.md).

REM-007 fitness: KEEP_LOCAL. Its accepted contract is clear, but consistency
across Selection's duplicated fields and partial versus complete traces is
more review-sensitive than the preceding weight patch. The parent will
implement it in action.rs, utility.rs, trace.rs and re-exports in lib.rs,
with directly affected tests and the two allowed reports. No other agent
owns these files. Constructor-call review found no production caller outside
sim-ai. This internal scheduling decision needs no additional user approval.

### REM-007 Code Accepted — Parent Implementation

Implemented the accepted three fallible constructors, shared complete/fragment
identity validation, validated serde and Selection copy correspondence. No
default weights, provider eligibility, source dependencies or mixer changed.
The parent observed both malformed-wire regressions fail before the fix;
workspace 163/163, 2 doctests, fmt, Clippy and rustdoc now pass. Existing
candidate/selection release smoke executables ran successfully. Detailed
contracts and test evidence: [CHRON-025](CHRON-025_ACTION_TRACE.md) and
[CHRON-026](CHRON-026_UTILITY_SCORING.md).

The post-fix Utility checksums exactly match the post-policy/pre-validation
baseline. Median times were 26.578/26.629 ms for 100 people (epsilon 0/25),
239.799/239.733 ms for 1,000. Whole-process max RSS was 8,503,296 B. The small
apparent timing improvement is not attributed to validation: both sides
include path checks, were compiled at different points and have uncontrolled
background/thermal noise. Final REM-008 rows retain exact output and limits.

Sensitive compatibility change: ActionCandidate, PerturbationSpec and
DecisionTrace constructors now return Result; malformed diagnostic JSON is
rejected instead of accepted/repaired. Valid wire keys and partial fragments
are preserved. No saved-game migration or remote publication occurred.

## 12. Historical Execution Handoff — Before REM-008A, 2026-08-30

The open measurement item in this checkpoint was subsequently included by the
owner and resolved in §13. This section preserves the earlier evidence, not a
current request for another tooling approval.

### Outcome

All planned code corrections and accepted ADR implementations are complete.
REM-008's executable checks, eight M5 benchmark workloads and per-task reports
are delivered. One measurement requirement remains unfulfilled: the existing
harnesses do not measure interval/per-workload **peak incremental RSS**.
Their retained deltas and newly recorded whole-command peaks are explicitly
distinguished, not relabeled to make the requirement pass. The remediation
batch is therefore not represented as unconditionally complete.

The user's whole-plan authorization is recorded once. There is no additional
routine Task/dispatch approval request, and no claim that the user has already
seen or accepted these final measured outcomes. CHRON-027 is outside this
identified plan and remains unstarted.

### Finding Disposition

| Finding | Final disposition | Evidence / remaining boundary |
|---|---|---|
| F-01 visibility/protection | Closed | Actual PUBLIC change and exact strict/admin two-check main protection independently read back; §11 records remote actions. |
| F-02 runtime ECS handle | Closed | Private test-only helper, original core tests and positive/compile-fail doctests pass. |
| F-03 Work starvation | Closed within selection scope | Accepted ADR-0018 table; old-table one-second failure reproduced; both full sweeps, low-pair/margin and reachable critical-need tests pass. Execution loop remains CHRON-027. |
| F-04 perturbation wire bypass | Closed | Native/range/spec/nested decoding rejects malformed epsilon, typed errors, no execution clamp. |
| F-05 malformed/ambiguous candidates | Closed | Shape, complete/partial identity, stable-key permutation and duplicated Selection copy checks pass under ADR-0019. |
| F-06 CHRON-018 review corrections | Closed, with disclosed audit-mechanism change | Six corrections and actual graph verified; four passing heuristic tests removed by approval. Future dependency changes require explicit metadata/tree review, not automatic audit enforcement. |
| F-07 original task authorization | Closed by owner clarification | No history rewrite or inference of an unauthorized implementation batch. |
| F-08 missing M5 evidence | Reports/timing resolved; peak-incremental metric remains open | Eight reports and full aggregate outputs now exist. Accurate isolated peak-incremental RSS needs follow-up measurement instrumentation; no written exception is assumed. |

### Final Verification Actually Run

- `./tools/ci-rust.sh`: immutable-spec hash; workspace fmt; warnings-denied
  Clippy; **163 unit/integration tests, zero failed/ignored**; Rust 1.95 MSRV;
  seven existing release smoke checks (headless, scheduler, 10K ECS, events,
  mode workload, SQLite and snapshot). All passed.
- `cargo test --workspace --doc`: 2 passed, including forbidden runtime handle.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`: passed.
- `./tools/ci-godot.sh`: Godot-rust loaded under Godot 4.7.2 without error.
- `gda script validate --all --project /Users/gabrielmu/Documents/Palimpsest/apps/macos-godot --json`:
  aggregate valid=true; main.gd, metrics_overlay.gd and tile_renderer.gd all
  valid, no diagnostics, correct project root.
- Exact dependency metadata and normal trees for sim-world/sim-ai reviewed
  again. Dependencies and Cargo.lock unchanged. `git diff --check` passed.
- Original test-function inventory in action.rs, trace.rs, utility.rs and
  person.rs was compared to HEAD: no original domain test function is missing.
  Only the separately approved four-test dependency audit was removed; it is
  recoverable from Git history. No test/lint/performance threshold was relaxed.

### M5 Evidence / Files

Apple M5 / 16 GiB / macOS 26.6.2; release builds, two warmups and ten samples
per final case. Eight benchmark commands exited successfully. Reports contain
exact aggregate stdout, commands, source identity, test coverage and limits:

- [CHRON-019 LocalGrid](CHRON-019_LOCAL_GRID.md)
- [CHRON-020 worldgen](CHRON-020_WORLDGEN.md)
- [CHRON-021 person runtime](CHRON-021_PERSON_RUNTIME.md)
- [CHRON-022 Needs](CHRON-022_NEEDS.md)
- [CHRON-023 activity sites](CHRON-023_ACTIVITY_SITES.md)
- [CHRON-024 pathfinding](CHRON-024_PATHFINDING.md)
- [CHRON-025 candidates/traces](CHRON-025_ACTION_TRACE.md)
- [CHRON-026 Utility scoring](CHRON-026_UTILITY_SCORING.md)

[PERFORMANCE.md](../PERFORMANCE.md) is the method/result index with dependency
versions and remediated source hashes. Each measurement has one authoritative
final section; older before/after smoke results are explicitly staged history.
REM-008 modified these reports, the CHRON-018 acceptance report, this ledger
and the performance index only. No benchmark or production code was optimized
during measurement.

### Sensitive Changes and Remaining Risks

1. Repository/history/accessible Actions information is PUBLIC. main enforces
   both approved checks strictly for administrators, with force-push/deletion
   disabled. License unchanged; no commit/push/merge performed.
2. The runtime ECS handle is no longer a public domain API. Stable EntityId
   access remains; the private handle is neither a persistent identity nor a
   new persistence mechanism.
3. Default Work availability weight is 2,300 and Eat/Sleep availability bonuses
   are zero. This fixes low-need preference without changing Needs growth,
   feasibility, trace factors or introducing execution.
4. Three constructors now return Result. Invalid candidate/perturbation/trace
   JSON is rejected; valid wire keys and partial diagnostic use remain. These
   are intentional accepted source/API changes, not save-file migrations.
5. Dependency checking changed from four heuristic automated tests to exact
   metadata/tree review. That review is required on dependency changes but is
   not automatic future CI enforcement.
6. A 100-person scoring round is about 26.5 ms in this fixture, excluding
   candidate preparation. The future kernel must budget decision cadence and
   path work; no every-frame-all-person decision assumption is justified.
7. Peak incremental RSS remains a measurement-tool gap, not a code failure or
   a relaxed budget. Follow-up must isolate a workload interval/baseline,
   record actual high-water memory (not only before/after samples), preserve
   ten-sample timing and correctness assertions, and distinguish per-scale
   results from whole-process maxima. It requires extending the measurement
   tooling beyond REM-008's report-only file whitelist; that unplanned edit was
   not silently bundled into this batch.

The working tree intentionally remains uncommitted, including pre-existing
owner changes. No unexpected generated artifact is tracked; no clean-tree or
new-commit claim is made. Local results cover the documented source hashes;
existing remote CI on e5b0aeb does not certify the unpushed remediation.
Master Spec is unchanged. No Phase 2 work, OpenCode run, external model API,
new sidebar task, CHRON-027 implementation or Phase 1 final acceptance occurred.

Final read-only GitHub recheck at handoff again returned PUBLIC, strict=true,
both approved check contexts, enforce_admins=true, allow_force_pushes=false
and allow_deletions=false. PR #1 remains Draft with head e5b0aeb676372a123dd8c27190e94b6a606d498c.

## 13. Approved Peak-Memory Extension Completed — 2026-08-30

The owner answered “纳入吧” to the proposed extension beyond REM-008's
report-only whitelist. That approval was recorded once in
[REM-008A](../tasks/REM-008A.md). Parent designed and recorded
[ADR-0020](../adr/ADR-0020-benchmark-memory-measurement.md), then dispatched
two bounded Luna adapter tasks while implementing the native runner locally.
No repeat dispatch/Task approval was requested.

F-08 is now **Closed**: 22 cases × three fresh processes have kernel-proved
cold and prepared-operation peak increments, with exact raw readings,
checksums and min/median/max. The transient 64 MiB probe and prior-peak
rejection tests pass. Old ps endpoint deltas and whole-command peaks remain
separate historical series; no performance budget or test was relaxed.

Authoritative evidence: [REM-008A completion report](REM-008A_MEMORY.md),
[raw memory samples](data/rem-008a-memory.jsonl) and
[follow-up timing series](data/rem-008a-timing.jsonl). CHRON-019..026 reports
and PERFORMANCE.md link this closure. Original timings use ten post-warmup
samples; cold memory uses three independent samples, not a replacement timing
protocol or a claim of warmed object size.

Parent verification: ./tools/ci-rust.sh passed **203 tests, zero failed or
ignored**, fmt, Clippy, MSRV and seven original release smokes; 2 doctests and
warnings-denied rustdoc passed; native CLI tests passed in debug and release;
all eight existing timing benchmarks completed. Exact metadata/tree review
confirmed unchanged simulation dependency sets. No original domain test or
benchmark statement was removed in this follow-up.

Sensitive changes in this extension:

1. One outward-only benchmark workspace binary has a narrowly documented
   native unsafe exception for Mach RSS reads and diagnostic-only fixed-size
   mappings. Production crates retain unsafe-forbid; domain APIs, allocator,
   ECS/identity/persistence/bridge contracts are unchanged.
2. The existing macOS CI job gains the native regression command because the
   Rust CI job runs on Linux. No existing checks/steps, permissions, job names
   or timeouts were weakened. The command passed locally; no remote run or
   branch-protection change is claimed.
3. Cargo.lock adds only this local tools package and its edges, reusing libc
   0.2.189 and all existing third-party versions. No new external model API.

Remaining limitations: macOS/page-granular RSS, three cold samples, first-use
and fixture overhead, and ordinary scheduling/thermal noise in short timing
benchmarks. The ~26 ms 100-person Utility round still informs the future kernel
cadence decision. This work does not implement CHRON-027, perform Phase 1's
100-NPC/10-year gate, or enter Phase 2.

The working tree is intentionally uncommitted. No push/merge, visibility or
remote-setting mutation occurred in REM-008A. Earlier changes remain preserved.
REM-009's final evidence review is not represented as already performed by the
owner merely because implementation was authorized; it is not an additional
per-subtask reconfirmation demand. CHRON-027 remains separately scoped.

## 14. REM-009 Acceptance Record — 2026-08-31

The product owner reviewed the REM-001..008A completion evidence and accepted
the remediation batch on 2026-08-31 ("验收REM-009"). Final dispositions:

- F-01 through F-08: **Closed** (F-06 closed with the disclosed
  audit-mechanism change: four approved heuristic audit tests removed in favor
  of exact `cargo metadata`/`cargo tree` review on dependency changes, which
  is not automatic future CI enforcement).
- ADR-0018 and ADR-0019 remain Accepted and are implemented as recorded.
- The Draft PR scope is understood; no unresolved critical finding is hidden.
- CHRON-027 was not auto-approved by this gate: it was separately authorized
  by the product owner's explicit approval of the P1-REMAINING /
  2026-08-30-r1 execution plan on 2026-08-30, and completed with
  [ADR-0021](../adr/ADR-0021-phase-1-action-execution-contract.md) and the
  [CHRON-027 report](CHRON-027_ACTION_STATE_MACHINE.md).

No code, test, budget, or remote-setting change accompanies this acceptance
record.
