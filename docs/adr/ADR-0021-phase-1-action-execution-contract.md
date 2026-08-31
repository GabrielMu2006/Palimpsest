# ADR-0021: Phase 1 Action Execution Contract

> Current supplement: [ADR-0025](ADR-0025-kernel-repair-completion.md).
> Historical decisions below are retained; use the supplement for V2 boundary repairs.

- Status: Accepted — approved with the P1-REMAINING / 2026-08-30-r1 execution
  approval on 2026-08-30
- Date: 2026-08-30
- Decision owners: Product owner
- Task: CHRON-027 in `docs/tasks/CHRON-027.md`; semantics fixed by
  P1-REMAINING D1 in `docs/PHASE_1_REMAINING_EXECUTION.md`
- Extends: ADR-0003, ADR-0004, ADR-0013, ADR-0014, ADR-0018, ADR-0019;
  does not supersede the Master Spec

## Context

CHRON-023..026 provide sites, pathfinding, validated candidates, traces, and
selection, but nothing owns the *lifecycle* of a selected action. CHRON-027
adds one authoritative execution state machine inside `palimpsest-sim-core`.
P1-REMAINING D1 fixes the semantics; this ADR records them with the exact
public contract before implementation. `sim-ai` keeps computing candidates and
scores only and receives no execution side effects; the executor consumes a
`Selection`/candidate produced from the same live context, never an imported
diagnostic JSON value (ADR-0019).

## Decision

### 1. Ownership and state shape

`ActionRuntime` (new module `crates/sim-core/src/actions.rs`) owns, per
person, at most one execution record, plus one `Scheduler<DueWork>`
(ADR-0004), a bounded in-memory outcome-event buffer, and monotonic counters.
Runtime execution state is never serialized and never crosses a
persistence/bridge boundary; `EntityId` remains the only identity (ADR-0002,
ADR-0011).

```rust
pub enum ActionState {
    Idle,
    Moving { action: ActionKind }, // movement phase of the recorded action
    Eating,
    Sleeping,
    Working,
}

pub struct Transition {
    // person, from, to, action kind, optional target, commit instant, reason
}

pub enum ActionError {
    UnknownPerson { id: EntityId },
    AlreadyExecuting { id: EntityId },
    InvalidTarget { kind: ActionKind },   // Idle targeted / targetless non-Idle
    Blocked { kind: ActionKind, target: LocalCoord },  // site missing/wrong kind
    Unreachable { kind: ActionKind, target: LocalCoord },
    Interrupted { id: EntityId },
    InvalidTransition { id: EntityId },
    Schedule { source: SchedulerError },
    EventLogExhausted,
}

pub struct ActionConfig {
    // move_seconds_per_cell: 1
    // eat: 600s, sleep: 28_800s, work: 1_800s, idle_wait: 60s
    // retry_delay: 1s, critical_recheck_delay: 60s
    // path: PathConfig::default()
}

pub struct ActionEnvironment<'a> {
    pub persons: &'a mut PersonRuntime,
    pub map: &'a WorldMap,
    pub sites: &'a mut ActivitySites,
}

impl ActionRuntime {
    pub fn new(config: ActionConfig) -> Self;
    pub fn start(&mut self, person: EntityId, action: ActionCandidate,
                 env: &mut ActionEnvironment<'_>, now: SimInstant)
        -> Result<Transition, ActionError>;
    pub fn advance(&mut self, now: SimInstant, env: &mut ActionEnvironment<'_>)
        -> Result<AdvanceOutcome, ActionError>;
    pub fn cancel(&mut self, person: EntityId, reason: CancelReason,
                  now: SimInstant, env: &mut ActionEnvironment<'_>)
        -> Result<Transition, ActionError>;
    pub fn current(&self, person: EntityId) -> Option<ActionState>;
    pub fn current_action(&self, person: EntityId)
        -> Option<(ActionKind, Option<LocalCoord>)>;
    pub fn next_due(&mut self) -> Option<SimInstant>;
    pub fn drain_events(&mut self) -> Vec<EventRecord>;
    pub fn stats(&self) -> ActionStats;
    pub fn metrics(&self) -> ActionRuntimeMetrics;
}
```

`advance` is queue-wide, not per-person: cross-person equal-instant ordering
is due-time then FIFO (ADR-0004), which a per-person entry point cannot honor.
`now` is only the drain horizon: every due item commits **at its own due
instant**, so one long advance is exactly equivalent to stepping through each
due instant separately (guarded by a segmentation-equivalence test).
`AdvanceOutcome` carries the committed `Vec<Transition>` and a
`Vec<DecisionRequest>` (`Completed` / `Retry` / `CriticalBoundary`, each with
person and instant). Decision requests are the only re-decision triggers; the
executor never scores or selects itself.

### 2. Timing and phases

- Movement costs 1 simulated second per 4-directional adjacent cell. The
  `find_path` result includes the start cell; execution begins at path index 1
  and never re-walks the start. No collision or occupancy exclusion exists;
  multiple persons may share a cell. No dynamic re-planning.
- Eat/Sleep/Work contain a movement phase: the person moves to the target,
  then enters `Eating`/`Sleeping`/`Working`. A standalone `Move` completes on
  arrival. Statistics record movement-phase completions (arrivals) and
  top-level action completions separately; no fabricated Move selection.
- A zero-distance targeted action still occupies one second: its arrival
  continuation is scheduled at `now + 1s`. Every action therefore occupies at
  least one simulated second, which structurally forbids same-instant
  completion loops.
- Durations: Eat 600s, Sleep 28,800s, Work 1,800s, Idle wait 60s
  (`ActionConfig` defaults; Phase 1 tuning values, not MVP balancing).
- On successful completion the executor first materializes needs growth up to
  the completion instant, then applies `Needs::eat(100_000)` /
  `Needs::rest(100_000)` for Eat/Sleep (saturating clamp at zero, per the
  existing `Needs` contract). Work completion only increments the site's
  bounded `WorkCounter` via `ActivitySites::record_work`. Interrupted,
  blocked, or failed actions receive no completion reward.
- Needs accrue from real elapsed simulated seconds exactly once per person:
  the runtime stores `last_needs_at` per person, commits
  `needs.advance(now - last_needs_at)` at every completion/cancel/check
  boundary, and never double-counts. Read views that need current values
  project through `advance` without committing.

### 3. Start validation, recovery, and interruption

- `start` on a person with an active record (including an Idle wait) returns
  `AlreadyExecuting` and changes nothing. Starting on an unknown person
  returns `UnknownPerson` and changes nothing. A structurally invalid
  kind/target combination returns `InvalidTarget`; contextual rechecks at
  start return `Blocked` (site missing or wrong kind) or `Unreachable`
  (`find_path` failure) without changing existing state. This is ADR-0019's
  executor boundary: structural validity does not prove contextual
  reachability, so the executor rechecks preconditions against simulation
  truth.
- Blocked/failed during execution (the arrival-time site recheck): cancel the
  record's live tokens, commit one atomic transition to `Idle`, and schedule
  a retry decision request at `now + retry_delay` (default 1s). No
  same-instant retry is possible.
- `cancel(person, CancelReason::Interrupted | CancelReason::External, ...)`
  materializes needs without reward, cancels both live tokens, commits one
  transition to `Idle`, and emits the corresponding outcome event.
  Interruption never bypasses Utility: the driver interrupts only when a fresh
  `select_action` on the live context elects a different `(kind, target)` than
  the executing action, and keeps the full `DecisionTrace`.
- Each person holds at most two live scheduler tokens: one action/retry
  continuation and one critical-need check. Replacing either cancels the old
  token first. Popped work whose token does not match the record's current
  token is discarded, so stale or double delivery cannot execute twice. Lazy
  stale heap nodes remain governed by ADR-0004 compaction.

### 4. Critical-need boundary checks

After every needs-affecting boundary the runtime schedules the person's next
critical check at the earliest instant a drive would reach
`CRITICAL_PRESSURE` (ceiling division on the committed raw values and rates).
When it fires, needs are materialized and a `CriticalBoundary` decision
request is emitted. If the person is still critical after the check handling,
the next recheck is `now + critical_recheck_delay` (default 60s): a positive
delay is mandatory, so a critical person cannot spin same-instant. No
emergency path bypasses the selector.

### 5. Events and counters

High-level outcomes — `action.completed`, `action.blocked`,
`action.failed`, `action.interrupted`, `action.cancelled` — are appended as
validated schema-1 `EventRecord` values (string `event_type`, `EntityId`
actor, `SimInstant` timestamp, metadata with kind/target/duration; ADR-0006,
ADR-0009-era schema unchanged). Idle completions are counted but not emitted:
they are pacing artifacts, not high-level outcomes. The buffer holds at most
4,096 records and drops oldest with a visible rotation counter
(`ActionRuntimeMetrics.events_rotated`); it is a bounded runtime diagnostic
sink, not durable retention. Event ids come from one monotonic `u64` counter;
exhaustion returns `EventLogExhausted` instead of panicking. `ActionStats`
carries per-kind completion counts, movement-phase completions, and
blocked/failed/interrupted totals as plain integers (no map iteration in
truth).

### 6. Determinism

Identical seed, world, decision sequence, and advance instants yield
byte-identical transition logs and event streams. The executor contains no
wall-clock, thread, float, or unordered-iteration dependence; ordering comes
from the scheduler's due-time/FIFO contract and stable `EntityId` keys.

## Consequences

- CHRON-028 composes this runtime with the clock and needs no second action
  owner; the closed-loop test in CHRON-027 drives it with a small test driver
  instead of the not-yet-built kernel.
- The ADR-0018 mandatory closed-loop integration test runs here: seed 25,025
  fixture, 172,800 seconds, executed twice with identical results.
- Godot never sees execution state directly; CHRON-029 projects it into the
  render DTO.

## Rejected / Deferred Alternatives

- Per-person `advance(person, now)`: rejected; it breaks cross-person
  due-time/FIFO ordering at equal instants.
- Zero-time actions completing inside `start`: rejected; same-instant
  completion loops become representable.
- A separate emergency action channel for critical needs: rejected; it would
  bypass the explainable selector (Master Spec §14, ADR-0014).
- Persisting execution state or emitting every decision trace to the Event
  Store: deferred; Phase 1 keeps bounded runtime diagnostics only.
- NPC collision/occupancy and dynamic re-planning: deferred; not Phase 1
  scope (P1-REMAINING D1).

## Task Completion / Acceptance Gate

- Dependencies: CHRON-023..026 implementations, accepted ADR-0018/0019, and
  the P1-REMAINING execution approval recorded 2026-08-30.
- Files: this ADR plus CHRON-027's allowed implementation/report surface.
- Tests and benchmark: per `docs/tasks/CHRON-027.md`, including the
  172,800-second closed loop and the transition/recovery matrix.
- Benchmark: `action_execution_bench` at 100 and 1,000 persons plus the
  REM-008A memory-tool adapters; results land in the CHRON-027 report.
- DoD: the semantics above hold in code; no weight, rate, schema, or
  persistence change is bundled.
