# CHRON-030 — Simulation Worker Command Bridge

> **Status: Implemented and locally verified 2026-08-31 — see
> [report](../reports/CHRON-030_SIMULATION_WORKER.md) and the
> [ADR-0015 Phase 1 supplement](../adr/ADR-0015-simulation-worker-command-render-snapshot.md).
> Implemented directly by the main agent (Kimi Code CLI); the handoff's Luna
> dispatch was unavailable in this runtime.**
> Execution handoff: [CHRON-030_START](../handoffs/CHRON-030_START.md). This approves030 only,
> including its necessary ADR/tests/measurement, not031–036 or remote publication.
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Provide a single Simulation worker that runs the CHRON-028 kernel and accepts a bounded stream of commands, advancing only at explicit tick boundaries, always publishing the latest complete Render Snapshot (CHRON-029), and supporting pause, speed scaling, and single-step. No IPC, no multi-threaded ECS.

## Context
ADR-0007 and ADR-0010 flagged that Phase 0 ran production simulation work synchronously on Godot's main thread and that Phase 1 must move that work to a Simulation worker with immutable, batched Render Snapshot publication. The product owner accepted the decision "spike a Simulation worker with immutable, batched Render Snapshot publication rather than running production simulation work on Godot's main thread; do not introduce a separate process yet." This Task delivers exactly that: one in-process worker (not a separate OS process) that owns the kernel, is driven by bounded commands, and hands the latest complete snapshot to the presenter, with pause/speed/step semantics. It is the bridge between CHRON-028 and the Godot presentation (CHRON-031).

## Scope
- Add one headless, in-process Simulation worker that owns a `WorldKernel` (CHRON-028) and is the only component allowed to mutate simulation state.
- Accept commands through a bounded channel/queue: `Pause`, `Resume`, `SetSpeed(multiplier)`, `Step(steps)`, `AdvanceTo(SimInstant)`, `Shutdown`.
- Enforce boundary semantics: `advance_to` may yield internally, but command application/publication waits for a complete due boundary. One kernel call is not necessarily a completed tick.
- Publish immutable complete snapshots; a read returns the latest already-published complete value at that read's synchronization point. Keeping that value until the next publication is valid; partial or backwards publication is not.
- Support initial pause (no automatic advancement), the D3 speed set and paused single-step (one simulation second). Explicit Step/AdvanceTo can advance a paused worker and leave it paused.
- Run the worker on one dedicated in-process thread for the Godot client; the ECS and kernel remain single-threaded on that worker. No thread pool, parallel ECS, async runtime, or OS process is introduced.
- Expose the same kernel and command semantics to headless callers; headless execution may drive the kernel directly and omit the presentation thread/snapshot exchange as allowed by ADR-0015.

## Out of Scope
- IPC, OS process boundary, sockets, shared memory, or network — explicitly deferred by the product owner.
- Multi-threaded execution of ECS systems, thread pools, async/await runtime, or parallel scheduling. The one dedicated Simulation worker thread is in scope.
- Input/rendering/UI in Godot (CHRON-031); this worker provides no presentation.
- Pause-on-significance, Watch, auto-pause, event-feed, or history retention.
- Persistence, save/load, snapshots as saves.
- resource economy, resources, production, needs-satisfaction content.
- LLM, NLG, war, politics, religion, magic.

## Dependencies
- CHRON-028 complete (kernel that owns time/ordering and exposes `advance`/tick boundary).
- CHRON-029 complete (immutable Render Snapshot DTO as the only rendered output).
- CHRON-006 Scheduler and ADR-0004 for due/FIFO ordering underlying each tick.

## Execution Steps / Readiness

1. Parent fixes the worker ADR: D3's 64-command queue, acknowledgement sequence,
   speed set, paused one-second Step, full-boundary visibility and shutdown.
2. Separate producer handle from worker-owned mutable kernel. The main thread
   cannot call kernel advance through a convenience method.
3. Test full queue plus shutdown, command rejection/ack, slow reader, incomplete
   due batch, and repeated lifecycle; exact snapshot hashes use explicit target
   commands, never a nondeterministic wall-clock arrival schedule.
4. Create worker/direct controls and memory adapter (§4). Use short controlled
   windows for pacing tests, not a real-time year. Parent owns concurrency;
   leaf tests/adapters may be delegated only after the contract is fixed.

## Files Modified / Allowed
- `crates/sim-core/**` (new `worker` module defining `SimulationWorker`, commands, and the bounded channel/queue).
- `crates/godot-bridge/**` only if a thin presentation method needs to drive the worker (the bridge is the Godot boundary per ADR-0007); otherwise prefer a Core-only worker.
- `apps/headless-runner/**` if the runner reuses the worker path.
- `docs/adr/ADR-0015-simulation-worker-command-render-snapshot.md` governs batching, backpressure, tick-boundary visibility, and snapshot ownership. A new ADR is required only if implementation diverges.
- `docs/reports/CHRON-030_SIMULATION_WORKER.md` for measured worker overhead/throughput.
- `docs/tasks/CHRON-030.md`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- A public worker type, e.g. `SimulationWorker`, exposing:
  - `new(kernel) -> Result<Self, WorkerError>`
  - `submit(command: WorkerCommand) -> Result<CommandSequence, WorkerError>` (bounded; `Full` is distinct, never silent drop; sequence identifies the later acknowledgement)
  - `drain_commands() -> ...` or an internal loop that applies commands at a tick boundary
  - `command_status(sequence) -> ...` (applied/rejected outcome and committed boundary; no public direct mutable-kernel access)
  - `latest_snapshot() -> Arc<RenderSnapshot>` (or equivalent owned immutable handle; newest complete publication at the read point)
  - `is_paused() -> bool`, `speed() -> SpeedMultiplier`, `step(steps) -> ...`
- `WorkerCommand` is a closed, bounded enum: `Pause`, `Resume`, `SetSpeed(SpeedMultiplier)`, `Step(u64)`, `AdvanceTo(SimInstant)`, `Shutdown`.
- `SpeedMultiplier` is the closed 1/5/20/100/1000/MAX set in D3. It changes pacing, not simulation cadence or truth. Invalid values return `InvalidSpeed`.
- `WorkerError` distinguishes `Full`, `InvalidSpeed`, `InvalidStep`/not paused, `ClockRegression`, `TickOverflow`, and `Closed`. Submitted commands receive sequence IDs; eventual application/rejection is observable, not inferred from enqueue success.
- Invariants to document in the ADR:
  1. One worker; simulation mutation only via the worker.
  2. Bounded command queue: `submit` reports `Full` rather than blocking unboundedly or dropping.
  3. Tick-boundary visibility: a submitted command takes effect only at the next committed tick boundary; readers never observe a partial/in-progress tick.
  4. Only the latest complete Render Snapshot is observable; a snapshot is never published mid-tick, the exchange owns at most two immutable snapshots, and consumers release obsolete handles rather than building history (ADR-0015).
  5. Pause, speed, and step semantics are deterministic and reproduce under a fixed schedule.
  6. The worker is single-threaded in-process here; IPC/multithreaded ECS is explicitly out of scope.

## Tests
- Bounded queue: saturating the queue returns `Full` and never drops or blocks unboundedly; after drain, submit succeeds.
- Tick-boundary visibility: a command submitted mid-tick is not observed until the next committed tick; no partial snapshot is observable.
- Pause: wall-driven work does not change `now()` while paused; resume continues from the exact boundary. Explicit validated Step/AdvanceTo are the only paused-time advance exceptions; AdvanceTo while running is rejected.
- Speed: each validated multiplier reproduces the correct sim-seconds-per-tick mapping; invalid values are rejected with `InvalidSpeed`.
- Step: while paused, 1 step advances one simulation second, drains all due work through that target and publishes a complete snapshot; zero is a no-op, above 1,000 or unpaused Step is rejected. An internal budget yield is not step completion.
- Latest-complete semantics: slow and concurrent readers see the newest complete publication at their read point, may retain it until a later publication, never see a partial batch or decreasing sequence. Snapshot retention stays bounded as in ADR-0015.
- Determinism: identical seed + identical command sequence yields identical final state and snapshot sequence.
- Shutdown: `Shutdown` leaves a clean, closed worker; later commands return `Closed`.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests, docs, dependency audit.

## Benchmark
- Worker throughput and overhead at 100 persons, release build, ten post-warm-up samples, median reported on M5 16GB.
- Report: commands processed per wall-second, sim-seconds per wall-second at each speed multiplier, latest-snapshot latency after a commit, peak process RSS delta, and max observed queue depth.
- Compare against a direct-kernel control to isolate worker overhead; assertions remain enabled; no budget relaxation.

## Definition of Done
- One in-process Simulation worker owns the kernel and mutates simulation only at whole tick boundaries.
- The command queue is bounded; saturation yields `Full`, never unbounded blocking or silent drop.
- Readers observe the latest published complete snapshot at the read point, never a partial or backwards publication; age/latency is measured rather than promised zero.
- Pause, speed (over a validated multiplier set), and single-step are deterministic and tested.
- No IPC, no separate process, no multithreaded ECS in this Task; this is explicitly recorded as deferred.
- Public worker contract and snapshot-publication ownership conform to ADR-0015; worker overhead is measured and documented.

## Required Completion Report
Report: change summary; commands run; benchmark result (commands/s, sim-seconds-per-wall-second per speed, latest-snapshot latency, RSS delta, max queue depth) with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; list of covered tests; known limitations (e.g., single-threaded in-process, no IPC/multi-thread ECS by design, no persistence); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
