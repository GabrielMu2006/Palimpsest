# CHRON-028 — Scheduler and Kernel Orchestration

> Final corrective verification/measurement: [repair V2 report](../reports/P1_KERNEL_REPAIR_V2.md).
> Use [CURRENT_PROGRESS](../CURRENT_PROGRESS.md); the original status below is historical.

> **Status: Implemented 2026-08-31 under the approved CHRON-028 Task; ADR-0022 accepted; repaired 2026-08-31 under ADR-0024.**
> This Task was separately approved by the product owner on 2026-08-31 and implemented
> with the ADR-0022 kernel contract; see `docs/reports/CHRON-028_KERNEL.md`.
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Wire the deterministic Simulation Clock, Scheduler, and runtime ECS entities into one authoritative headless kernel that advances a world of 100 Persons through deterministic due-work boundaries, with no Godot involvement.

## Context
Phase 0 proved the pieces in isolation: `SimClock`, `Scheduler<Scheduled<T>>`, stable `EntityId`, and structured events. Phase 1 now needs a single owner that decides *when* systems run. Without a kernel, each system would pick its own cadence and the world would not have a deterministic "tick boundary" against which CHRON-027's action machine, CHRON-030's command bridge, and CHRON-032's 10-year runner can all agree. This Task is the seam between the proved primitives and a running 100-NPC world.

## Scope
- Add a headless, deterministic kernel that owns a `SimClock`, a `Scheduler`, and the person/terrain runtime entities and wiring established by CHRON-019..022.
- Provide `advance_to(target, work_budget)` for due work through a requested `SimInstant`, with explicit Reached/Yielded progress. Each fixed Phase 1 system has declared cadence; no per-second full-AI scan or speculative plugin registry.
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

## Execution Steps / Readiness

1. Parent records the new kernel ADR and exact progress/error contract using
   P1-REMAINING D2, including a 1,024-due-instant-round default budget. A full round may process many items and is not a wall-time bound (ADR-0024/0025).
2. Compose the actual 027 state machine; do not duplicate its action semantics.
   Add read-only counters/logic-state digest inputs needed by 029/032/033 now.
3. Test segmentation/budget invariance, equal-time FIFO, lazy Needs and fatal
   error progress; drive an actual 100-Person day before forecasting year costs.
4. Create `kernel_bench`, memory adapter and runner entry; measure a year using
   P1-REMAINING §4. Kernel ordering and integration are parent-owned work.

## Files Modified / Allowed
- `crates/sim-core/**` (new `kernel` module; may absorb/centralize the world runtime).
- `Cargo.toml`, `Cargo.lock` only if a workspace member needs to change; prefer `sim-core` module addition.
- ADR-0003/0004/0011/0013/0017 remain in force. Record the newly introduced kernel progress/commit/cadence contract in a new ADR before implementation.
- `docs/reports/CHRON-028_KERNEL.md` is the required kernel evidence report.
- `docs/tasks/CHRON-028.md`.
- Any benchmark harness files it needs under `apps/headless-runner` or `benchmarks/`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- A public kernel type, e.g. `WorldKernel`, that exposes:
  - `new(config) -> Result<Self, KernelError>` (fixed Phase 1 composition, not a plugin/system registry)
  - `now() -> SimInstant`
  - `advance_to(to: SimInstant, work_budget: usize) -> Result<KernelAdvance, KernelError>`
  - `person_count() -> usize`
  - `person(id: EntityId) -> Result<Option<KernelPersonView>, KernelReadError>` (complete-boundary accessor; ADR-0024/0025)
  - `process_due(now) -> ProcessedWork` (the per-boundary primitive)
- `KernelAdvance` reports requested target, actual committed progress, `Reached` or `Yielded`, processed work and event count. Exhausting a valid budget yields resumable progress; zero/invalid budgets are explicit errors, not silent no-work success.
- `KernelError` distinguishes clock regression, invalid budget, arithmetic exhaustion and system failure, with last complete boundary information.
- Due items execute in due-time/FIFO order exactly once. Each item commits atomically; a call may yield with earlier items already committed. It never promises rollback of previously committed history. A same-instant partial batch cannot be published as a complete boundary. Repeating the same target drains remaining due work; regression fails without mutation. No due work at or before the target remains only when status is `Reached`.
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
- Single-boundary, multi-boundary, budget-yielded and empty-world execution terminate. `Reached(T)` has no remaining due work through T; Yielded keeps it pending. One large call and repeated small budgets yield identical truth and ordered event digests.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests, docs, dependency audit.

## Benchmark
- Deterministic advance throughput at 100 persons over bounded smoke and one-year intervals, release build, ten post-warm-up samples, median reported on M5 16GB. The authoritative 10-year run belongs to CHRON-032.
- Report advancement seconds-of-sim per wall-second, processed work per second, peak process RSS delta, maximum scheduler queue depth, and events/s produced.
- This is the primary kernel throughput baseline for CHRON-032/033. No numerical sim-speed minimum has been approved; report measured speed, not an invented pass threshold. The 100-person correctness, memory and presentation requirements remain unchanged.

## Definition of Done
- A single headless kernel owns clock + scheduler + person/terrain wiring and advances deterministically to a requested `SimInstant`.
- Only due systems execute in due-time/FIFO order; system cadence is explicit; clock regression and equal-target behavior are guarded.
- 100 persons run and are addressable by stable `EntityId`; the kernel is the sole ordering authority.
- The kernel is exposed to headless runner, chaos runner, and the CHRON-030 worker through one API path.
- The implementation conforms to the time, scheduler, ECS, action, and crate-boundary ADRs; kernel benchmark results are reproducible and documented.

## Required Completion Report
Report: change summary; commands run; benchmark result (advance TPS/processed-work/s, sim-seconds-per-wall-second, RSS delta, queue depth, events/s) with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; list of covered ordering regression scenarios; known limitations (e.g., single-threaded, no persistence, 100-person Phase 1 gate only); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
