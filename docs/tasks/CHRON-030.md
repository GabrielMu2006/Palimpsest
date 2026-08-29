# CHRON-030 — Simulation Worker Command Bridge

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Provide a single Simulation worker that runs the CHRON-028 kernel and accepts a bounded stream of commands, advancing only at explicit tick boundaries, always publishing the latest complete Render Snapshot (CHRON-029), and supporting pause, speed scaling, and single-step. No IPC, no multi-threaded ECS.

## Context
ADR-0007 and ADR-0010 flagged that Phase 0 ran production simulation work synchronously on Godot's main thread and that Phase 1 must move that work to a Simulation worker with immutable, batched Render Snapshot publication. The product owner accepted the decision "spike a Simulation worker with immutable, batched Render Snapshot publication rather than running production simulation work on Godot's main thread; do not introduce a separate process yet." This Task delivers exactly that: one in-process worker (not a separate OS process) that owns the kernel, is driven by bounded commands, and hands the latest complete snapshot to the presenter, with pause/speed/step semantics. It is the bridge between CHRON-028 and the Godot presentation (CHRON-031).

## Scope
- Add one headless, in-process Simulation worker that owns a `WorldKernel` (CHRON-028) and is the only component allowed to mutate simulation state.
- Accept commands through a bounded channel/queue: `Pause`, `Resume`, `SetSpeed(multiplier)`, `Step(steps)`, `AdvanceTo(SimInstant)`, `Shutdown`.
- Enforce tick-boundary semantics: work is applied in whole kernel `advance()` intervals; a command's effect is visible only at the next committed tick boundary; no partial/inside-tick mutation.
- Always publish the *latest complete* Render Snapshot (CHRON-029), never a partial or stale one; a reader receives the newest fully-committed snapshot, not an in-progress tick.
- Support pause (no advancement), speed scaling (a bounded set of multipliers applied as sim-seconds per wall-second), and single-step (advance exactly one tick interval).
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

## Files Modified / Allowed
- `crates/sim-core/**` (new `worker` module defining `SimulationWorker`, commands, and the bounded channel/queue).
- `crates/godot-bridge/**` only if a thin presentation method needs to drive the worker (the bridge is the Godot boundary per ADR-0007); otherwise prefer a Core-only worker.
- `apps/headless-runner/**` if the runner reuses the worker path.
- `docs/adr/ADR-0015-simulation-worker-command-render-snapshot.md` governs batching, backpressure, tick-boundary visibility, and snapshot ownership. A new ADR is required only if implementation diverges.
- `docs/reports/CHRON-030_SIMULATION_WORKER.md` for measured worker overhead/throughput.
- `docs/tasks/CHRON-030.md`.
- No product doc changes; no `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` edits without a Change Proposal.

## API Contract
- A public worker type, e.g. `SimulationWorker`, exposing:
  - `new(kernel) -> Result<Self, WorkerError>`
  - `submit(command: WorkerCommand) -> Result<(), WorkerError>` (bounded; `Full` is a distinct error, never silent drop)
  - `drain_commands() -> ...` or an internal loop that applies commands at a tick boundary
  - `advance(to: SimInstant) -> Result<KernelAdvance, WorkerError>` (applies all pending commands then advances)
  - `latest_snapshot() -> Arc<RenderSnapshot>` (or equivalent owned immutable handle; always the newest complete committed snapshot)
  - `is_paused() -> bool`, `speed() -> SpeedMultiplier`, `step(steps) -> ...`
- `WorkerCommand` is a closed, bounded enum: `Pause`, `Resume`, `SetSpeed(SpeedMultiplier)`, `Step(u64)`, `AdvanceTo(SimInstant)`, `Shutdown`.
- `SpeedMultiplier` is a bounded, validated set (e.g. a closed set of allowed multipliers), rejecting any out-of-range value with `InvalidSpeed`.
- `WorkerError` distinguishes `Full` (bounded queue saturated), `InvalidSpeed`, `ClockRegression`, `TickOverflow`, and `Closed`.
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
- Pause: while paused, `advance`/wall-driven calls do not change `now()`; resume continues from the exact boundary.
- Speed: each validated multiplier reproduces the correct sim-seconds-per-tick mapping; invalid values are rejected with `InvalidSpeed`.
- Step: `step(1)` advances exactly one tick interval and produces exactly one newer complete snapshot; `step(0)` is a no-op or documented error.
- Latest-complete semantics: interleaving multiple `advance`s guarantees the reader always sees the newest complete snapshot, never a stale or partial one.
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
- Readers always observe the latest complete Render Snapshot, never a stale or partial tick.
- Pause, speed (over a validated multiplier set), and single-step are deterministic and tested.
- No IPC, no separate process, no multithreaded ECS in this Task; this is explicitly recorded as deferred.
- Public worker contract and snapshot-publication ownership conform to ADR-0015; worker overhead is measured and documented.

## Required Completion Report
Report: change summary; commands run; benchmark result (commands/s, sim-seconds-per-wall-second per speed, latest-snapshot latency, RSS delta, max queue depth) or explicit N/A; list of covered tests; known limitations (e.g., single-threaded in-process, no IPC/multi-thread ECS by design, no persistence); and any blocker. Do not auto-start the next Task; each requires separate product-owner approval.
