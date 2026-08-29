# ADR-0015: Simulation Worker / Render Snapshot Boundary

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
