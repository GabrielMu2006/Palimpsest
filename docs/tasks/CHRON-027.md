# CHRON-027 — Action Execution State Machine

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

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
- CHRON-021, CHRON-022 established the person entity and basic movement that this machine must drive.

## Files Modified / Allowed
- `crates/sim-core/**` (new `world`/`actions` module and any kernel-adjacent modules it introduces).
- `Cargo.toml`, `Cargo.lock` only if a new internal crate/workspace member is required (prefer adding a module to `sim-core` instead).
- `docs/adr/ADR-0013-person-needs-action-boundaries.md` and `docs/adr/ADR-0014-explainable-utility-decision-contract.md` govern the execution and trace boundaries. A new ADR is required only if implementation must diverge from them.
- `docs/reports/CHRON-027_ACTION_STATE_MACHINE.md` for any recorded measurement/limitation, if produced.
- `docs/tasks/CHRON-027.md`.
- No other product/documentation file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` unless a genuine conflict requires a Change Proposal first.

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
  4. On any `Blocked`/`Unreachable`/`Failed` outcome the machine recovers to Idle deterministically and releases all Scheduler tokens it held.
  5. The machine never invokes resource-economy logic or blocks the simulation tick on decision/content.

## Tests
- Each legal transition is exercised and asserted; no transitive/invalid transition is admitted.
- Overlap prevention: attempting to start a second action while one is active returns `AlreadyExecuting` and leaves the person unchanged.
- Blocked/unreachable recovery: a person that becomes unable to reach/perform returns to Idle and its Scheduler tokens are released (no token leakage; scheduler metrics show zero stale/held entries attributable to the machine).
- Interrupt: a higher-priority needs event aborts a running action and commits exactly one transition.
- Determinism: identical seed + same action sequence yields byte-identical transition log for a fixed tick schedule.
- Terminal handling: after completion the person returns to a legal state and can accept a new action.
- Structured-event emission: high-level outcomes produce valid bounded in-memory `EventRecord`s that pass `validate()` with stable `EntityId`/`EventId` references; decision traces do not enter the durable Event Store.
- Workspace gates: fmt, Clippy with warnings denied, debug and release workspace tests, docs, and dependency audit.

## Benchmark
- Headless action-transition throughput at 100 and 1,000 persons over a fixed simulated interval, release build, ten post-warm-up samples, median reported on the M5 16GB reference machine.
- Report transitions/s, peak process RSS delta, and any Scheduler queue growth during the run.
- Correctness assertions remain enabled; no budget relaxation is inferred.

## Definition of Done
- Move, Eat, Sleep, Work, and Idle execute and advance over time with atomic, single-commit transitions.
- Blocked/unreachable/interrupted/failed cases recover to Idle deterministically with no dangling runtime state, no ScheduleToken leaks, and no stuck entities.
- Meaningful transitions emit valid structured events; simulation truth is never invented by presentation.
- The action machine is headless and independent of Godot and LLM, and does not implement resource economy.
- Public transition/execution contract conforms to ADR-0013/0014; tests and (if run) benchmark results are reproducible and documented.

## Required Completion Report
After finishing, the implementer must report: the exact change summary; the commands actually run; transitioning benchmark results or an explicit N/A with reason; the list of covered transition and recovery scenarios; any known limitations (e.g., no pathfinding, no economy, single-action-per-person); and any blocker. This Task is not automatically followed by the next Task; the product owner must approve each separately.
