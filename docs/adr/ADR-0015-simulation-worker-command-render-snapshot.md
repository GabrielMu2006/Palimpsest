# ADR-0015: Simulation Worker / Render Snapshot Boundary

> CHRON-030 supplement below fixes the concrete thread, acknowledgement,
> shutdown, error, and pacing contract for the Phase 1 worker implementation.

- Status: Accepted by product-owner Phase 0 decision (2026-08-29, decision 6)
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for Simulation worker or bridge-concurrency changes

## Context

Phase 0's mode probe ran Simulation and Render Snapshot publication synchronously
on Godot's main thread, and the headless/rendered results differed by 2.09×
(ADR-0010). Risk 6 in the final report flags main-thread bridge use: Phase 1 must
decide how Simulation execution and Render Snapshot publication avoid frame
stalls without prematurely introducing a separate process or multi-threaded ECS.

## Decision

Run production Simulation Core on a single Simulation worker and never on Godot's
main thread. Publication is via immutable, batched Render Snapshots.

- One Simulation worker drives the kernel; it owns the authoritative world and
  scheduler. Godot main thread never runs a core tick.
- A bounded command queue carries Phase 1 client intent (pause, time-speed,
  single-step, and initial world config) into the worker. The queue is
  bounded to prevent unbounded growth and to keep the worker responsive.
- The worker publishes a `RenderSnapshot`: an immutable, latest-complete view of
  simulation state. Use a bounded latest-value exchange with at most two
  exchange-owned snapshots; the presenter holds only its current frame snapshot
  and releases the prior one. Slow rendering drops obsolete presentation
  snapshots, never simulation commands or events.
- Godot reads only the immutable `RenderSnapshot` (render-oriented, batched); it
  cannot mutate Simulation Core (ADR-0007).
- Inputs and command effects take effect only at tick boundaries, so a render
  frame or in-flight command never mutates mid-tick simulation state.
- Headless execution uses exactly the same kernel and command semantics but may
  drive the kernel directly without a presentation worker thread. Godot is not
  present and snapshot publication is optional in headless mode.
- Phase 1 introduces no separate process (no IPC) and no multi-threaded ECS.

## Public Contract

- `SimulationWorker` produces a shared, latest-complete immutable
  `RenderSnapshot` and accepts bounded commands; it is the only mutator of the
  authoritative World.
- A `RenderSnapshot` is a read-only, cloneable view model, not an inner handle
  into World internals.
- The Godot main thread consumes `RenderSnapshot` only; rendering cannot write
  simulation state.
- Command effects are visible only on the next applied tick.
- Snapshot exchange is finite (at most two exchange-owned immutable snapshots) and
  never exposes shared mutable ECS state to the reader. Consumers must not
  retain an unbounded history of snapshot handles.

## Consequences

- Render frame stalls from full-core ticks are avoided; headless remains
  independent of rendering (2.09× indicator preserved).
- A single worker keeps the deterministic kernel single-threaded and simple to
  reason about, holding the Phase 1 scheduler contract (ADR-0004).
- Safety and ownership need explicit care: the snapshot hand-off must be
  structured so Godot's read of the immutable buffer and the worker's write to
  the other buffer never overlap the same mutable slot. The implementation uses
  safe Rust synchronization; ADR-0007's `unsafe` restriction is unchanged.
- Render LOD / streaming of snapshots remains a later phase; the buffered batch is
  a single local map in Phase 1.
- Separate process IPC and multi-threaded ECS are explicitly out of Phase 1; a
  superseding ADR is required if a future scale gate needs them.

## Rejected / Deferred Alternatives

- Run Simulation on Godot's main thread: rejected; it stalls frames and was the
  identified Phase 0 risk.
- Separate OS process / IPC boundary: deferred; unnecessary complexity for Phase
  1, and the product-owner decision explicitly says no separate process yet.
- Multi-threaded ECS in Phase 1: rejected; global parallelism needs measurement
  and would conflict with the deterministic single-worker baseline (ADR-0004).
- Polling fresh render state every frame without immutable batching: rejected; it
  risks mid-tick reads, frame stalls, and duplicated work.

## Supersedes / Extends

Extends ADR-0007 (Godot GDExtension boundary) with the Phase 1 concurrency and
publication model, and resolves the ADR-0010 flagged synchronous main-thread
probe. Supersedes the "synchronous mode probe" default for production execution
without changing ADR-0010's measurement-only scope.

## Phase 1 Supplement — CHRON-030 Worker Contract (2026-08-31)

- Status: Accepted within the product owner's explicit CHRON-030 approval on
  2026-08-31 (see `docs/handoffs/CHRON-030_START.md`). The handoff's Luna
  subagent dispatch was unavailable in the executing environment (Kimi Code
  CLI has no `gpt-5.6-luna` spawn tool); per the dispatch skill's own fallback
  rule the main agent implemented the task directly and this is reported as a
  limitation, not a silent substitution.
- Extends the Decision above with concrete bounds and failure semantics; it
  does not change ADR-0021/0022/0023/0024/0025 simulation semantics.

### Threading and ownership

- `SimulationWorker::new(kernel)` spawns exactly one dedicated `std::thread`
  that owns the `WorldKernel` exclusively; all simulation mutation happens on
  that thread. Safe Rust, standard library only (`std::sync::mpsc`,
  `Mutex`, atomics); no thread pool, async runtime, IPC, or new dependency.
- `new` rejects a faulted kernel (`KernelFaulted`) and a non-empty `Setup`
  kernel that was never started (`KernelNotStarted`), because the first forward
  advance would otherwise surface `KernelError::NotStarted` mid-command.
  `new` blocks until the worker thread has built and published the **initial
  snapshot** (publication sequence 1); a failed initial build returns the
  error instead of a half-alive worker.
- The worker starts **paused**. Headless callers may still drive a kernel
  directly without the worker, as the Decision already allows.

### Commands, sequencing, and acknowledgements

- `WorkerCommand` is the closed set `Pause`, `Resume`,
  `SetSpeed(SpeedMultiplier)`, `Step(u64)`, `AdvanceTo(SimInstant)`,
  `Shutdown`. `SpeedMultiplier` is the closed set `1/5/20/100/1000/MAX`
  (`SpeedMultiplier::from_u32` maps 1/5/20/100/1000 and rejects everything
  else with `InvalidSpeed`; `MAX` is a dedicated variant with no numeric
  factor).
- The command queue is a bounded `sync_channel` with capacity **64**
  (`COMMAND_QUEUE_CAPACITY`). `submit` returns `Full` when saturated and
  `Closed` after shutdown; it never blocks unboundedly and never silently
  drops. A `CommandSequence` (monotonic `u64` from 1) is consumed only by a
  successfully enqueued command.
- The worker applies commands **only between kernel calls**, i.e. at a
  complete committed boundary; a command submitted mid-advance takes effect at
  the next boundary after the in-flight call returns. Publication likewise
  happens only between kernel calls, so a reader never observes a partial
  tick.
- Every enqueued command produces exactly one `CommandAck` recording its
  sequence, the command, `Applied`/`Rejected(WorkerError)`, and the actual
  committed boundary after application. Rejections are side-effect-free and
  are never presented as success. Acks are retained in a bounded log of the
  latest **1,024** (`ACK_LOG_CAPACITY`); `command_status` reports `Pending`,
  `Completed(ack)`, `Evicted` (older than the retained window), or `Unknown`
  (never assigned).
- `Step`/`AdvanceTo` keep calling the kernel's bounded `advance_to` until the
  target is actually reached; an internal budget yield is never reported as
  completion. The ack's `committed_to` is the real boundary, so a preempted or
  faulted advance cannot masquerade as having reached the target.

### Pause, step, advance-to

- `Pause` halts wall-driven advancement; it does not block later explicit
  `Step`/`AdvanceTo`. `Resume` continues from the exact committed boundary.
- `Step(n)`: only while paused, else `InvalidStep`. `n == 0` is a
  side-effect-free no-op (no advance, no publication). `n > 1_000`
  (`MAX_STEP_STEPS`) is rejected `InvalidStep`. A valid step advances exactly
  `n` simulation seconds and leaves the worker paused. `now + n` overflow is
  rejected `TickOverflow` with no mutation.
- `AdvanceTo(t)`: only while paused, else `NotPaused`. `t < now` is rejected
  `ClockRegression`; `t == now` is a no-op. A valid command advances to
  exactly `t` using the unchanged kernel semantics and leaves the worker
  paused.

### Speed and pacing

- Speed changes **pacing only**: simulation cadence, weights, LOD, and content
  are untouched; `MAX` never skips simulation work to gain speed.
- For a numeric multiplier `m` the worker keeps an anchor `(wall_instant,
  sim_instant)` reset on `Resume` and `SetSpeed`, and targets
  `anchor_sim + floor(elapsed_wall_ms * m / 1000)`; when caught up it sleeps
  until the next due simulated second, capped at 50 ms so the independent stop
  path stays responsive. `MAX` advances toward `now + 31,536,000 s` per call
  (each call still bounded by the kernel work budget) without wall-clock
  waits. Wall-clock pacing is not a reproducible input trace and never enters
  simulation truth.

### Publication and snapshot ownership

- The exchange holds **one** slot (`Mutex<Option<(sequence, Arc<RenderSnapshot>)>>`);
  with the reader's current frame this is at most two exchange-owned immutable
  snapshots, satisfying the Decision's bound. Publication replaces the slot
  atomically under a monotonically increasing publication sequence starting
  at 1; `latest_snapshot` returns the newest complete publication at the read
  point, never a partial or older one.
- Publication is forced on the initial snapshot, `Pause`, a non-zero `Step`,
  a non-no-op `AdvanceTo`, and shutdown; while running it is throttled to at
  most once per 100 ms of wall clock (the 10 Hz target). Latency is measured,
  never promised zero.

### Fault and shutdown

- A kernel error during advancement moves the worker to `Faulted`: the cause
  is exposed via `status()`, no new DTO is built, the last complete
  publication is retained, and `Resume`/`Step`/`AdvanceTo` are rejected
  `KernelFaulted`. `Pause`/`SetSpeed`/`Shutdown` remain accepted. There is no
  rollback or recovery path (ADR-0024 D3).
- Shutdown has two paths: the queued `Shutdown` command, and an independent
  atomic stop flag set by `SimulationWorker::shutdown()`/`Drop`, which works
  even when the queue is full (no re-enqueue needed). On exit the worker
  drains remaining queued commands as `Rejected(Closed)`, publishes a final
  snapshot if the committed boundary advanced since the last publication, and
  marks the phase `Closed`; later `submit` returns `Closed`. A stop-flag
  preemption aborts an in-progress `Step`/`AdvanceTo` between kernel calls and
  acks it `Rejected(Closed)` with the actual committed boundary.
- `WorkerError` distinguishes `Full`, `Closed`, `InvalidSpeed`, `InvalidStep`,
  `NotPaused`, `ClockRegression`, `TickOverflow`, `KernelFaulted`, and
  `KernelNotStarted`.

### Deferred

- IPC, a separate OS process, multi-threaded ECS, persistence, and
  pause-on-significance remain out of Phase 1 scope as recorded above.
