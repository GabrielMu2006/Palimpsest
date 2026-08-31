# ADR-0022: Phase 1 Scheduler / Kernel Orchestration Contract

> Current supplement: [ADR-0025](ADR-0025-kernel-repair-completion.md).
> Historical decisions below are retained; use the supplement for V2 boundary repairs.

- Status: Accepted — approved by the product owner with CHRON-028 on 2026-08-31
- Date: 2026-08-31
- Decision owners: Product owner
- Task: CHRON-028 in `docs/tasks/CHRON-028.md`; semantics fixed by
  P1-REMAINING D2 in `docs/PHASE_1_REMAINING_EXECUTION.md`
- Extends: ADR-0003, ADR-0004, ADR-0011, ADR-0013, ADR-0017, ADR-0021;
  does not supersede the Master Spec

## Context

Phase 0 proved `SimClock`, `Scheduler<Scheduled<T>>`, stable `EntityId`, and
structured events in isolation. CHRON-021/022/027 supplied the person runtime,
needs, and the action execution state machine, but no single component owns
*when* systems run: without a kernel, each system would choose its own cadence
and the world would lack a deterministic tick boundary against which the
action machine, the command bridge (CHRON-030), and the 10-year chaos runner
(CHRON-032) can all agree. This ADR records the kernel as the sole owner of
time advancement and ordering, with a bounded, progress-reporting advance API
and kernel-owned structured-event accounting. `sim-ai` keeps computing
candidates/scores; the kernel only invokes the already-accepted action runtime
as due work and resolves the decision requests it surfaces.

## Decision

### 1. Ownership and state shape

`WorldKernel` (new module `crates/sim-core/src/kernel.rs`) owns the clock, the
static local world, the activity sites, the person runtime, the identity
allocator, the action runtime, the decision weights/perturbation, the latest
per-person decision trace, and a bounded kernel event buffer. Runtime execution
state and scheduler tokens are never serialized; `EntityId` remains the only
cross-boundary identity (ADR-0002/0011).

```rust
pub struct KernelConfig {
    pub action: ActionConfig,
    pub weights: Weights,
    pub perturbation: PerturbationSpec,
    pub work_budget: usize,          // default 1_024 advance rounds per call
    pub event_buffer_capacity: usize, // default 4_096
}

pub struct WorldKernel { /* clock, map, sites, persons, allocator, actions,
                              weights, perturbation, work_budget, decisions,
                              events, ... */ }

impl WorldKernel {
    pub fn new(map: WorldMap, sites: ActivitySites, config: KernelConfig) -> Self;
    pub fn from_world(seed: WorldSeed, config: KernelConfig) -> Self; // generate + place_defaults
    pub fn now(&self) -> SimInstant;
    pub fn spawn_person(&mut self, location: LocalCoord) -> Result<EntityId, KernelError>;
    pub fn start_world(&mut self, at: SimInstant) -> Result<usize, KernelError>;
    pub fn advance_to(&mut self, target: SimInstant, work_budget: usize)
        -> Result<KernelAdvance, KernelError>;
    pub fn person_count(&self) -> usize;
    pub fn person(&self, id: EntityId) -> Option<KernelPersonView>;
    pub fn persons(&self) -> impl Iterator<Item = KernelPersonView>;
    pub fn latest_trace(&self, id: EntityId) -> Option<&DecisionTrace>;
    pub fn drain_events(&mut self) -> Vec<EventRecord>;
    pub fn metrics(&self) -> KernelMetrics;
}
```

`add_actor`/metadata: the kernel pushes only high-level action outcome events.
`start_world` runs `decide_and_start` for every spawned person at `at`; it is
the only seed step, so every person enters advance already holding an active
action (at least an `Idle` wait). The kernel never invents a decision: it only
resolves the `DecisionRequest` values the action runtime surfaces.

### 2. Bounded advance with explicit progress

`advance_to(target, work_budget)` jumps between due instants rather than
scanning every person every second:

- Reject `target < now` with `KernelError::ClockRegression` and no mutation.
- Repeat: for the earliest due instant `d <= target`, run
  `actions.advance(d, env)` (all due work at exactly `d`, due-time/FIFO,
  ADR-0004), then resolve every surfaced `DecisionRequest` with the configured
  weights/perturbation via `resolve_decision`. Each due instant processed is
  one "advance round"; `rounds >= work_budget` halts the loop early.
- One long `advance_to` is exactly equivalent to splitting the same target
  across several calls (segmentation equivalence): within a call no work is
  skipped or duplicated, and each round commits at its own due instant.
- `KernelAdvance` reports `committed_to` (the last committed instant), the
  `rounds` executed, `reached_target` (false when the budget was exhausted
  before the horizon), and the committed `transitions`/`decisions`/`events`
  counts. Budget exhaustion is a resumable progress point, not a fake success.
- When no more work is due and `target` is reached (or no work exists), the
  clock advances to `target` and `reached_target` is true.
- At most one fully committed snapshot of each tick is observable: the kernel
  drains the action event buffer into its own bounded buffer only after a
  full due-instant round commits (D2).

### 3. Per-person decision traces and event accounting

- The kernel stores only the **latest** complete `DecisionTrace` per person
  (a `BTreeMap<EntityId, DecisionTrace>`, bounded by person count), so
  Developer Mode can answer "why" without retaining a 10-year log.
- High-level action outcome events are validated, counted, and appended to a
  bounded buffer (`KernelMetrics.events_rotated` tracks overflow). This is a
  runtime diagnostic sink, not durable retention or Event Store history.

### 4. Determinism

Identical `(world seed, config, spawn sequence, advance sequence)` yields
byte-identical ordered event/decision summaries and identical final visible
state. The kernel contains no wall-clock, thread, float, or
unordered-iteration dependence; ordering comes from the scheduler due-time/FIFO
contract, the stable deterministic candidate/selection path, and stable
`EntityId` key ordering.

## Consequences

- CHRON-030 (worker) and CHRON-032 (chaos runner) call the same
  `advance_to` path in-process; the headless runner reuses the kernel API.
- CHRON-029 builds its versioned render DTO from the kernel's read-only
  boundary and never supplies its own `now`.
- The `run_until` reference driver in action.rs is superseded by the kernel
  for multi-person worlds; it remains as the single-person CHRON-027 closed
  loop harness.

## Rejected / Deferred Alternatives

- Per-second full-person scan: rejected; violates system cadence and the
  event-driven invariant (ADR-0004, P1-REMAINING D2).
- Unbounded `advance` with no budget: rejected; a malformed world could hang.
- Auto-kicking idle persons during advance: rejected; it would re-decide a
  person already waiting on a positive retry delay, bypassing the surfaced
  `DecisionRequest` (ADR-0021 §3).
- Rolling back committed ticks on error: deferred; the kernel stops at the last
  fully committed boundary and reports it.
- Persisting the kernel or retaining all decision traces: deferred; Phase 1
  keeps bounded runtime diagnostics only.

## Task Completion / Acceptance Gate

- Dependencies: CHRON-021/022/027 implementations and accepted ADRs;
  P1-REMAINING execution approval recorded with the plan.
- Files: this ADR plus CHRON-028's allowed implementation/report surface.
- Tests and benchmark: per `docs/tasks/CHRON-028.md`, including determinism,
  budget/reached semantics, cadence, and bounded-event accounting. The
  100-NPC/10-year run belongs to CHRON-032, not this gate.
- DoD: the semantics above hold in code; no weight, rate, schema, or
  persistence change is bundled.
