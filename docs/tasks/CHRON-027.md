# CHRON-027 — Action Execution State Machine

> Final corrective verification/measurement: [repair V2 report](../reports/P1_KERNEL_REPAIR_V2.md).
> Use [CURRENT_PROGRESS](../CURRENT_PROGRESS.md); the original status below is historical.

> **Status: Implemented 2026-08-30 under the approved P1-REMAINING / 2026-08-30-r1 plan; repaired 2026-08-31 under ADR-0024.**
> Contract: ADR-0021. Evidence: `docs/reports/CHRON-027_ACTION_STATE_MACHINE.md`.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Context
Phase 1 needs a small set of Person actions that actually move the Person through the world. The Phase 1 build-up Tasks (CHRON-023..026) provide sites, pathfinding, action contracts, and Utility AI primitives, but none of them is responsible for the *lifecycle* of an action: accepting a selected candidate, executing it over an interval, detecting completion/interruption/block/failure, and recovering to Idle without corrupting the entity. Without a single authoritative execution state machine, movement and needs systems would impinge on one another and "100 NPC live for 10 years" (CHRON-032) could not be validated.

## Objective
Implement one deterministic, headless action-execution state machine that owns the runtime state of a Person's current action, performs atomic legal transitions, and recovers safely from blocked or failed execution. It must support Move, Eat, Sleep, Work, and Idle. It must not introduce resource/economy quantities.

## Scope
- Add a headless, Godot/LLM-free action state machine within Simulation Core.
- Model a Person action as an explicit runtime state with a stable, bounded transition set: `Idle`, `Moving`, `Eating`, `Sleeping`, `Working`, and terminal/abort states.
- Provide execution drivers that advance a current action over `SimInstant` time using the Scheduler (CHRON-006).
- Require atomic transitions: no action may silently overlap another; every state change is a single committed transition.
- Define blocked (target unavailable / path unreachable), interrupted (higher-priority need supersedes), completed, and failed outcomes.
- On failure or block, recover deterministically to `Idle` (or a documented permissible state) without leaving dangling runtime state, leaked ScheduleTokens, or stuck entities.
- Emit bounded structured events for high-level action outcomes (completed, blocked, failed) into the kernel's in-memory event sink. Per-decision traces remain runtime diagnostics and are not appended to the durable Event Store in Phase 1.
- Keep resource/economy (food stock, currency, production) entirely out of scope; action completion is time- and state-based only.

## Out of Scope
- Resource economy, production chains, inventory counts, prices, or storage.
- Personality, Values, Skills, Relations, Memory, Goals, or Knowledge.
- Utility AI scoring itself (provided by CHRON-026); this Task only executes the action it selects.
- Pathfinding implementation (provided by CHRON-024); this Task only consumes paths and observes unreachable/budget outcomes.
- ECS selection or `bevy_ecs` integration decisions; reuse whatever runtime handle layer CHRON-021+ established.
- Anything Godot-facing, rendering, or LLM.
- War, politics, religion, magic, historians, NLG.

## Dependencies
- CHRON-023, CHRON-024, CHRON-025, and CHRON-026 complete (sites, pathfinding, action/trace contracts, and Utility selection).
- CHRON-006 Scheduler (due-time payloads complete).
- CHRON-009 structured events available for transition realism.
- CHRON-021/022 provide PersonRuntime, Location and Needs, not an implemented movement loop. This Task supplies movement execution.
- Accepted ADR-0018/0019 and completed REM-008A are the current scoring/validation/measurement baseline.

## Execution Steps / Readiness

1. Parent fixes the execution ADR under P1-REMAINING D1: timing, need satisfaction,
   action-versus-movement-stage counts, interrupts, tokens, errors and exact signatures.
   This is part of this Task, not another owner approval.
2. Reuse `PersonRuntime`, `Needs::advance/eat/rest`, current fallible candidates,
   `Path` and Scheduler; implement private current-action storage plus stable-ID views.
3. Run the 172,800-second real selector/executor test required by ADR-0018, then
   the negative cases below. It uses a test driver, not the not-yet-built 028 kernel.
4. Create `action_execution_bench` and its memory-tool adapters before claiming
   RSS results; run the commands/protocol in P1-REMAINING §4 and report evidence.
   Design/closed-loop integration stays parent-owned; fixed-contract leaf tests
   may be delegated when requested, with independent review.

## Files Modified / Allowed
- `crates/sim-core/**` (new `world`/`actions` module and any kernel-adjacent modules it introduces).
- `Cargo.toml`, `Cargo.lock` only if a new internal crate/workspace member is required (prefer adding a module to `sim-core` instead).
- ADR-0013/0014/0018/0019 govern existing boundaries; add an execution-contract ADR before introducing the new public state machine. Do not change accepted weights/rates or weaken native/serde validation.
- `docs/reports/CHRON-027_ACTION_STATE_MACHINE.md` is required, including measurements and limits.
- `docs/tasks/CHRON-027.md`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- A public action runtime type, e.g. `ActionRuntime`, owning at most one current action per person and exposing:
  - `start(person, action, environment) -> Result<Transition, ActionError>`
  - `advance(person, now, environment) -> TransitionResult`
  - `cancel(person, reason) -> Transition`
  - `current(person) -> Option<ActionState>`
- An `ActionState` enum with a bounded, non-overlapping set of variants (Idle, Moving, Eating, Sleeping, Working, plus an explicit terminal/abort marker as needed).
- `Transition` is a single atomic result recording old state, new state, the `ActionKind`, and the `SimInstant` at which it was committed.
- `ActionError` distinguishes `Blocked`, `Unreachable`, `Interrupted`, `InvalidTransition`, and `AlreadyExecuting`.
- Invariants to document and enforce via ADR/contract:
  1. A person has at most one active action at any `SimInstant`.
  2. No state change is observable until the transition is committed atomically.
  3. Runtime execution state (the action machine's own token/handle) is never persisted across snapshot boundaries; stable `EntityId` remains the only cross-boundary identity.
  4. Execution `Blocked`/`Unreachable`/`Failed` recovers to Idle and cancels held live tokens. Invalid start/overlap instead leaves the existing action unchanged. Lazy stale heap nodes remain governed by ADR-0004, not a promise of instant physical removal.
  5. The machine never invokes resource-economy logic or blocks the simulation tick on decision/content.

## Tests
- Each legal transition is exercised and asserted; no transitive/invalid transition is admitted.
- Overlap prevention: attempting to start a second action while one is active returns `AlreadyExecuting` and leaves the person unchanged.
- Blocked/unreachable recovery: no attributable live token remains after abort; stale nodes stay within ADR-0004 compaction bounds and disappear on compact. Repeated cancel and old-token delivery cannot execute twice.
- Interrupt: a higher-priority needs event aborts a running action and commits exactly one transition.
- Determinism: identical seed + same action sequence yields byte-identical transition log for a fixed tick schedule.
- Terminal handling: after completion the person returns to a legal state and can accept a new action.
- Structured-event emission: high-level outcomes produce valid bounded in-memory `EventRecord`s that pass `validate()` with stable `EntityId`/`EventId` references; decision traces do not enter the durable Event Store.
- Closed loop: actual candidate generation, scoring, movement, completion and Needs updates yield positive Work/Eat/Sleep completions, corresponding need reductions and a return to Work, as specified by ADR-0018 and P1-REMAINING D1. Selected actions alone do not prove execution.
- Workspace gates: fmt, Clippy with warnings denied, debug and release workspace tests, docs, and exact normal-dependency review.

## Benchmark
- Headless action-transition throughput at 100 and 1,000 persons over a fixed simulated interval, release build, ten post-warm-up samples, median reported on the M5 16GB reference machine.
- Report transitions/s, peak process RSS delta, and any Scheduler queue growth during the run.
- Correctness assertions remain enabled; no budget relaxation is inferred.

## Definition of Done
- Move, Eat, Sleep, Work, and Idle execute and advance over time with atomic, single-commit transitions.
- Blocked/unreachable/interrupted/failed cases recover to Idle deterministically with no dangling runtime state, no ScheduleToken leaks, and no stuck entities.
- Meaningful transitions emit valid structured events; simulation truth is never invented by presentation.
- The action machine is headless and independent of Godot and LLM, and does not implement resource economy.
- Public transition/execution contract conforms to ADR-0013/0014/0018/0019 and its new execution ADR; all required tests and benchmarks, including the closed loop, have reproducible evidence.

## Required Completion Report
After finishing, the implementer must report: the exact change summary; the commands actually run; transitioning benchmark results with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; the list of covered transition and recovery scenarios; any known limitations (e.g., no new pathfinding algorithm, no economy, single-action-per-person); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
