# CHRON-032 — Headless 10-Year Chaos Runner

> **Status: Proposed — covered by P1-REMAINING / 2026-08-30-r1; implementation not yet approved.**
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Run a 100-entity, fixed-seed headless world continuously for 10 simulated years through the CHRON-028 kernel and the action/needs loop, validating the Phase 1 DoD ("continuous simulation for 10 years without crashing") and producing a structured report confirming no panic, no NaN, no infinite loop, no dangling reference, and no unbounded queue.

## Context
`MASTER_SPEC.md` §84 makes "continuous 10 years without crash" the Phase 1 Definition of Done; §76 requires a Chaos Simulation Test proving no instant extinction, no infinite resource growth, no NaN, no infinite loop, no dangling Entity references, database consistency, and no obvious memory leak. This Task is the concrete headless vehicle for that proof. It needs the kernel (CHRON-028) so the action/needs loop (CHRON-023..027) actually executes over years, and a structured, reproducible runner so CHRON-034 can rerun it in CI and CHRON-033 can time it at scale.

## Scope
- Add a headless Chaos Runner (bin + API) that seeds a 100-person world and advances the CHRON-028 kernel continuously for 10 simulated years.
- Fix the seed and World Config provided to the runner so the entire run is deterministic and replayable.
- Exercise the full Phase 1 action/needs loop: persons move, eat, sleep, work, and idle (CHRON-027) over years, interleaved with needs (CHRON-022), pathfinding (CHRON-024), and Utility AI selection (CHRON-026).
- Instrument the run to detect and fail on: panics, `NaN`/non-finite values (in position, needs, utility, action state), infinite or non-terminating loops, dangling `EntityId`/`Entity` references, and unbounded Scheduler queue growth.
- Assert that all 100 Phase 1 Persons persist because Phase 1 has no death system, and that no modeled quantity grows unboundedly (no resource economy exists, so assert action/needs/positions/counters remain within their declared bounds at sampled checkpoints).
- Produce a structured machine-readable report (JSON plus Markdown) containing: population over time, aggregate and per-person action distribution, scheduler queue max/mean, events produced, sim-seconds-per-wall-second, peak RSS, and any invariant violation. Death statistics are explicitly `NotApplicable` in Phase 1.
- Feed its report into CHRON-033 (performance) and CHRON-034 (regression CI), and supply the M5 16GB reference-measured duration for CHRON-036.

## Out of Scope
- Resource economy, production, storage, or financial quantities (Phase 3).
- Ageing, birth, and death systems (Phase 2). Phase 1 removes no Person during this run.
- Family, memory, relations, personality, skills, profession content (Phase 2+).
- Persistence of the run (this is an in-memory validation run); no save/load.
- Rendering, Godot, animation, UI.
- LLM, NLG, war, politics, religion, magic.
- Asserting a performance *budget* claim beyond reporting the measured duration; the hard gate here is correctness/no-crash, not speed.

## Dependencies
- CHRON-028 complete (kernel that owns time/ordering and the 100-person world).
- CHRON-021..027 are transitively complete through CHRON-028; they provide Person, needs, sites, pathfinding, utility, and action execution.
- CHRON-006/CHRON-009 provide Scheduler and bounded structured events used by the runner.

## Execution Steps / Readiness

1. Parent fixes D4's seed-42 100-Person fixture, 315,360,000-second horizon,
   invariant bounds and canonical hash/streaming-digest schema in an ADR.
2. Implement runner + explicit CLI, instrument both successful completions and
   movement stages. Stream/check diagnostics without retaining all events/traces.
3. Prove detectors with test-only fixtures/hooks or invalid decoding; do not
   weaken private domain validation to inject an impossible native state.
4. Run day smoke, forecast full-run cost, then three full ten-year runs with
   equal deterministic outputs. The memory-tool wrapper measures the same
   workload; sampled checkpoint RSS is labeled trend, not an exact peak.
   Fixed CLI/report leaves may be delegated; invariant review stays parent-owned.

## Files Modified / Allowed
- `apps/headless-runner/**` (new chaos-runner bin; reuse the headless CLI where sensible).
- `crates/sim-core/**` (any runner-adjacent diagnostic/validation helper, and invariant-check instrument that must live in Core, not Godot).
- `docs/reports/CHRON-032_CHAOS_10YEAR.md` for the structured result.
- `docs/tasks/CHRON-032.md`.
- Optionally a fixture under `tests/worlds/` for the fixed 100-person seed corpus if CHRON-034 will reuse it.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- A public headless entry, e.g. `run_chaos(config: ChaosConfig) -> Result<ChaosReport, ChaosError>`, where:
  - `ChaosConfig { seed: u64, person_count, years, sim_seconds_per_year }` (person_count defaults to 100; years defaults to 10).
  - `ChaosReport` is structured and serializable (JSON), containing: final `SimInstant`, person-count-over-time samples, per-tick action/needs finite checks, scheduler queue min/max/mean, event count, sim-seconds-per-wall-second, peak process RSS, and a list of violated invariants (empty on success).
- The runner must return an error (non-zero exit) on any of: `Panic`, `NonFinite`, `NonTerminating`, `DanglingReference`, `QueueGrowth`, or `Invariant`.
- Invariant checks are deterministic assertions against documented thresholds, not ad hoc warnings; thresholds are fixed and justified in the report.
- The same config yields the same simulation-state hash, invariant samples, action counts, and event sequence across runs. Wall-clock duration and RSS are measurement fields excluded from deterministic equality/hash comparisons.
- The runner and its checks are headless and Godot-independent.

## Tests
- Determinism: two runs with the same seed/config produce identical deterministic report fields and final-state hash; timing/RSS fields are compared only as measurements. At least one fixed alternate seed must produce a different world/final hash.
- 10-year completion: the run reaches the configured end instant without error on the M5 reference machine.
- Panic/NaN/loop/dangling/queue detectors fire and fail the run when a deliberately injected violation is present.
- Population preservation: person count remains exactly 100 throughout because no Phase 1 death/removal system exists.
- Post-run invariants: at Reached(end) no due work through end remains, but future live entries are expected and bounded by D2. Check stale-compaction bounds, EntityId resolution and all produced-event validation; do not require an actively scheduled world to be empty.
- Finite/bounded quantities: position, needs, action state, and utility inputs stay finite and within documented bounds at every sampled tick.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests, docs, dependency audit.

## Benchmark
- Three complete independent 10-year/100-person runs on M5 16GB, release build, with separate short smoke warm-up; record all durations and min/median/max. Exact seed/count/horizon and commands follow P1-REMAINING §4, not a reduced-duration substitute.
- Report wall-clock duration, sim-seconds-per-wall-second, peak process RSS delta, max scheduler queue depth, total events produced, and event/s.
- This is a correctness evidence run; report the duration and note that CHRON-033 (100/1K/3K/5K/10K) and CHRON-036 handle budget claims.

## Definition of Done
- A fixed-seed 100-person world runs headless through the kernel for 10 simulated years and completes without a panic, NaN, infinite loop, dangling reference, or unbounded queue.
- All 100 Persons persist and all sampled quantities stay finite and bounded. Each Person must complete movement and Eat/Sleep/Work at least once. Movement may be a real completed pre-activity stage; report that separately from top-level Move selection/completion. Idle state must be observed in aggregate. This does not permit teleportation, manufactured selections or counting a selected action as completed.
- Database consistency is recorded as `NotApplicable`: the Phase 1 chaos run is deliberately in-memory and does not claim Event Store/save durability.
- The runner emits a structured, deterministic, reproducible `ChaosReport` (JSON + report doc) with invariant evidence.
- Seed/config determinism is demonstrated; deliberate-violation tests prove the detectors work.
- The headless Core remains authoritative and Godot-independent; the run is in-memory (no persistence required).

## Required Completion Report
Report: change summary; commands run; benchmark result (wall-clock, sim-seconds-per-wall-second, peak RSS, max queue depth, events, event/s) with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; the fixed seed/delay and covered invariants; list of covered detectors and violation tests; known limitations (e.g., no economy/ageing/death, in-memory only, 100-person Phase 1 gate); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
