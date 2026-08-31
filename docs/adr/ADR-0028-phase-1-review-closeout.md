# ADR-0028 — Phase 1 review repairs and closing evidence

- Status: Accepted implementation contract under the owner's 2026-08-31 instruction
  to fix the reviewed issues with Codex Luna and complete CHRON-033–036.
- Scope: repairs to 030–032, then the existing 033–036 dependency order. No Phase 2.
- Extends ADR-0015/0026/0027. No simulation cadence, weights, identities, history,
  unsafe boundary or performance budget changes.

## Worker and presentation

Keep the bounded FIFO command queue and one immutable publication slot. An
explicit long advance is interruptible at a committed kernel-call boundary when
a queued Pause or Shutdown arrives: acknowledge the unfinished advance as
`Rejected(Interrupted)` at its actual boundary, then process queued commands in
FIFO order. Independent/queued shutdown rejects subsequent work as Closed.
An internal budget yield with no command continues, never falsely completes.
Publish throttled intermediate complete snapshots during explicit long drives.
A publication carries its sequence, snapshot, construction-start and publish
monotonic timestamps atomically. A batched worker observation reads publication
and current status under the same lock order; status.committed may be newer than
the published snapshot and is labeled separately. Presentation publication number
always comes from that publication. Age measures time since snapshot construction
began, not a guessed bound from the publication frequency. Wall timestamps never
enter simulation truth/serialization. Existing latest_snapshot/status remain.
Godot measures monotonic frame intervals, snapshot_frame call and node-refresh
cost, retains raw frames and reports FPS percentiles separately from frame-time
percentiles. Missing RSS or latency is never replaced by zero.

## Chaos diagnostics

`WorldKernel::observations()` exposes a fallible read-only `KernelObservations`
with an ordered per-person table of `PersonObservations` (movement_steps,
movement_phases, moves, eats, sleeps, works, idles) and boundary sampling counters.
Counts fold existing committed transitions, including decision resolutions,
without changing transitions/events. Only Arrived transitions whose from AND to
are Moving represent a real last cell traversal; the following activity-arrival
transition is not counted twice. Step counts cells; zero-distance arrival does
not count movement. Completed transitions count top-level actions. Observations
are O(population), reject reads after faults, and survive event-buffer rotation.

Preserve `run_chaos(config, require_all_kinds)` and add
`build_chaos_kernel(config)` and `run_chaos_observed(config, require_all_kinds,
observer)`; observer gets a `ChaosCheckpoint` (Prepared, Advance, Day, Complete)
and a shared kernel reference. This is read-only instrumentation, never a truth
input. `run_chaos` is its no-observer wrapper. Core has no wall-clock/RSS code.
Validate full population identity/cardinality, bounds, action consistency,
event actors AND targets/record structure, monotone progress and final next_due.
Report unknown RSS as null. Compare all deterministic report fields (measurement
excluded), not only hashes. Version the strengthened hash schema; include actions,
full per-person observations, checkpoints, next due and deterministic counters.
Idle remains reported separately (ADR-0027), never manufactured by changing AI.

The CLI runs Core on a supervised std thread and waits for progress with an
inactivity timeout. A single stuck call or panic yields nonzero exit; no recovery
is claimed. The memory tool wraps the same observed chaos function, retaining
native ADR-0020 interval proofs and separate daily current-RSS trend samples.
No new dependency or unsafe capability is required.

## Evidence economy and acceptance

The owner reported one expensive RSS run and asked to avoid protective repeats.
Preserve the existing three ten-year timings as historical evidence for their
source. The existing one-RSS kernel-10-year sample is valid for its colocated
fixture, not the spread chaos fixture. Run one corrected full ten-year chaos
with native RSS and daily trend together; reuse unchanged simulation truth
counters/event digest for comparison and repeat short deterministic corpus runs.
This is an explicit reduced-repeat validation decision, not three new ten-year
runs or an assertion that old timings measure new diagnostics overhead.
033 still uses its short one-day sequential scale sweep (2 warmups,10 timings);
RSS can use one documented cold sample per scale under the owner's preference.
Critical targeted regressions and final combined local/hosted gates are retained;
do not rerun the entire suite at every leaf handoff. Historical files stay intact.
Publication follows the existing identified P1-REMAINING candidate branch/Draft PR
step only, with no merge, main write, force push or settings change. 036 states
real limitations and stops for Phase 1 owner acceptance.

## Read-only measurement counters and fixture alignment

SchedulerCounters records successful enqueue/dequeue/cancel/reschedule operations;
stale heap pops are not live dequeues. ActionRuntime and WorldKernel only forward
these counts. No ordering, cadence, or truth changes. Queue min/sum/max and count
are sampled at successful advance returns, explicitly not per-item peak claims.
The Godot 100-person fixture now uses the same existing deterministic reachable
spawn resolver as chaos, instead of colocating all sprites. The population cap
and simulation rules are unchanged. This also makes later mode comparisons use
identical initial state.
