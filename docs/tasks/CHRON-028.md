# CHRON-028 — Scheduler and Kernel Orchestration

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Wire the deterministic Simulation Clock, Scheduler, and runtime ECS entities into one authoritative headless kernel that advances a world of 100 Persons through deterministic due-work boundaries, with no Godot involvement.

## Context
Phase 0 proved the pieces in isolation: `SimClock`, `Scheduler<Scheduled<T>>`, stable `EntityId`, and structured events. Phase 1 now needs a single owner that decides *when* systems run. Without a kernel, each system would pick its own cadence and the world would not have a deterministic "tick boundary" against which CHRON-027's action machine, CHRON-030's command bridge, and CHRON-032's 10-year runner can all agree. This Task is the seam between the proved primitives and a running 100-NPC world.

## Scope
- Add a headless, deterministic kernel that owns a `SimClock`, a `Scheduler`, and the person/terrain runtime entities and wiring established by CHRON-019..022.
- Provide a single `advance()` (or equivalent) that processes Scheduler work due at or before a requested `SimInstant`. The kernel must not unconditionally run full AI for every entity for every simulated second; each registered system declares its cadence/query behavior.
- Establish deterministic commit boundaries at requested/due instants. Work is processed by `due_time` then FIFO order (ADR-0004); no universal one-second gameplay tick is introduced.
- Wire the kernel to the person/terrain runtime layer from CHRON-021+ so 100 Persons are spawned, enumerated for the kernel, and addressable by stable `EntityId`.
- Make the kernel the sole authority for ordering: systems receive only the due payloads and a read-only `SimClock`; they cannot reorder or invent time.
- Expose a headless API usable by the headless runner (CHRON-007), the CHRON-032 chaos runner, and the CHRON-030 worker, so all three exercise the same execution path.
- Record the kernel boundary and causality/ordering invariants in an ADR (public cross-module API + ECS wiring decision).

## Out of Scope
- Godot, GDExtension, rendering, input, or Scene Tree truth.
- Any `bevy_ecs` adoption/replacement debate beyond what CHRON-021+ already decided; this Task only wires the existing runtime layer.
- Utility AI scoring, action selection, and action transitions (CHRON-025/CHRON-027) — this Task only invokes them as due systems.
- Economy, resources, needs satisfaction, production, or any Phase 3 content.
- Multi-threading, async runtime, parallel ECS, background workers, or IPC.
- Memory/LOD policy, history retention, persistence, or snapshots.

## Dependencies
- CHRON-021, CHRON-022 complete (Person runtime and Needs established and addressable by stable `EntityId`).
- CHRON-027 complete (the action state machine that the kernel drives each tick).
- CHRON-005 SimClock, CHRON-006 Scheduler, CHRON-009 structured events, CHRON-004 `EntityId` (all proven in Phase 0).
- Terrain/local-tile/pathfinding dependencies (CHRON-019..024) are transitively complete through CHRON-027.

## Files Modified / Allowed
- `crates/sim-core/**` (new `kernel` module; may absorb/centralize the world runtime).
- `Cargo.toml`, `Cargo.lock` only if a workspace member needs to change; prefer `sim-core` module addition.
- ADR-0003, ADR-0004, ADR-0011, ADR-0013, and ADR-0017 govern time, ordering, ECS, action boundaries, and crate ownership. A new ADR is required only if implementation must change those public decisions.
- `docs/tasks/CHRON-028.md`.
- Any benchmark harness files it needs under `apps/headless-runner` or `benchmarks/`.
- No product/architecture/perf doc changes without a Change Proposal; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md`.

## API Contract
- A public kernel type, e.g. `WorldKernel`, that exposes:
  - `new(world, clock, scheduler, systems) -> Result<Self, KernelError>`
  - `now() -> SimInstant`
  - `advance(to: SimInstant) -> Result<KernelAdvance, KernelError>`
  - `person_count() -> usize`
  - `person(id: EntityId) -> Option<...>` (read-only accessor for diagnostics/render)
  - `process_due(now) -> ProcessedWork` (the per-boundary primitive)
- `KernelAdvance` reports the ending `SimInstant`, number of systems run, total due work processed, and any produced structured-event count.
- `KernelError` distinguishes `ClockRegression`, `WorkBudgetExceeded`, and `SystemFailure`.
- Deterministic boundary contract: `advance(to)` processes work due at or before `to`, in due-time/FIFO order, never runs the same scheduled item twice, and commits only after the bounded due batch succeeds. `advance(now)` is a documented no-op after processing any already-due work; advancing backward fails without mutation.
- Invariants to document in the ADR:
  1. The kernel, not any system, owns time advancement and ordering.
  2. Systems are invoked only when due. A registered system may perform a deterministic ECS query, but the kernel does not unconditionally full-scan every Person for every simulated second.
  3. The clock is monotonic; a backward target returns `ClockRegression` and an equal target follows the documented no-op/already-due rule.
  4. Work is bounded: due processing yields to the caller; there is no unbounded inner loop unless the caller requests it.
  5. The kernel is headless and has no LLM, Godot, or economy dependency.

## Tests
- Deterministic tick ordering: systems/actions fire in exactly the documented order for a fixed seed and tick schedule; repeat run is byte-identical.
- Equal-target and clock-regression behavior follows the documented contract without partial mutation.
- Only due work runs: scheduling work in the future never fires it early; equal-time work is FIFO.
- 100-Person kernel: spawn 100 persons, advance a bounded interval, assert every person is addressable by `EntityId`, that a snapshot of current actions/positions is stable, and that no spanning event crosses a tick boundary inconsistently.
- Cadence invariant: instrument system invocations to prove the kernel does not run every system for every simulated second and that registered due queries execute only at their cadence.
- Single-boundary, multi-boundary, and empty-world execution all terminate; after advancing to `T`, no work due at or before `T` remains, while future work may remain queued.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests, docs, dependency audit.

## Benchmark
- Deterministic advance throughput at 100 persons over bounded smoke and one-year intervals, release build, ten post-warm-up samples, median reported on M5 16GB. The authoritative 10-year run belongs to CHRON-032.
- Report advancement seconds-of-sim per wall-second, processed work per second, peak process RSS delta, maximum scheduler queue depth, and events/s produced.
- This is the primary Phase 1 kernel throughput number that CHRON-032/CHRON-033 will compare against; the 100-person result is the Phase 1 hard gate and must not be relaxed.

## Definition of Done
- A single headless kernel owns clock + scheduler + person/terrain wiring and advances deterministically to a requested `SimInstant`.
- Only due systems execute in due-time/FIFO order; system cadence is explicit; clock regression and equal-target behavior are guarded.
- 100 persons run and are addressable by stable `EntityId`; the kernel is the sole ordering authority.
- The kernel is exposed to headless runner, chaos runner, and the CHRON-030 worker through one API path.
- The implementation conforms to the time, scheduler, ECS, action, and crate-boundary ADRs; kernel benchmark results are reproducible and documented.

## Required Completion Report
Report: change summary; commands run; benchmark result (advance TPS/processed-work/s, sim-seconds-per-wall-second, RSS delta, queue depth, events/s) or explicit N/A; list of covered ordering regression scenarios; known limitations (e.g., single-threaded, no persistence, 100-person Phase 1 gate only); and any blocker. Do not auto-start the next Task; each requires separate product-owner approval.
