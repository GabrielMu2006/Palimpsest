# CHRON-033 — Representative Scale Benchmarks

> **Status: Complete — reference measurements verified 2026-08-31 under the owner’s explicit033–036 instruction.**
> Evidence: [scale report](../reports/CHRON-033_SCALE_BENCHMARKS.md); ADR0028/0029.
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Measure the Phase 1 micro world on the M5 16GB reference machine at 100 / 1K / 3K / 5K / 10K persons, reporting RSS, simulation throughput (TPS/advance rate), scheduler, pathfinding, utility, events, and snapshot/bridge metrics where each is applicable. The 100-person result is the Phase 1 hard gate; the higher scales are diagnostic and must not be used to relax the 3 GB / 5 GB / 7 GB memory caps.

## Context
`MASTER_SPEC.md` §73-75 define M5 16GB as the first performance contract, 100–200 NPC as the MVP target with a 60 FPS normal-UI and 200-year headless goal, and §74 sets provisional caps of <3GB (MVP core+client), <5GB (10K), <7GB (with Tiny LLM). It also mandates a Performance Test Suite of `bench_100/1k/3k/5k/10k_entities` plus `bench_event_store`, `bench_snapshot`, `bench_relationship`, `bench_memory`, `bench_utility_ai`, `bench_pathfinding`, `bench_nlg`, `bench_history_query`. Phase 0 measured only a dummy two-component workload; Phase 1 must re-measure with a representative person/kernel workload that includes needs, utility AI, movement/scheduler, actions, and render snapshot/bridge cost. The Architecture Spike report (ADR-0005) explicitly requires re-evaluation "after representative components, Scheduler integration, snapshots, and rendered/headless comparison." This Task is that re-measurement and must not relax any budget.

## Scope
- Build a benchmark harness that runs the representative Phase 1 world (kernel + needs + utility + movement + action + scheduler) at person counts 100, 1,000, 3,000, 5,000, and 10,000 on the M5 16GB reference machine using release builds.
- Report, for each scale: peak RSS (delta over control), throughput/advance rate (sim-seconds-per-wall-second and processed-work/s), scheduler enqueue/dequeue depth and throughput, utility AI decision throughput, pathfinding cost where present, structured-event generation throughput, and Render Snapshot serialization/build cost.
- Report bridge overhead where the Godot bridge/snapshot path is applicable, and the worker command/snapshot latency where the CHRON-030 worker is applicable; keep rendered and headless measurements separate (CHRON-017 showed headless ~2.09× faster).
- Treat 100 as the Phase 1 hard gate: the 100-person result must meet the Phase 1 budget (no budget relaxation), but no guarantee is made for the higher scales other than diagnostic reporting.
- Record methodology per `docs/PERFORMANCE.md`: release builds, warm-up, sample count, exact command, dependency versions, median + variance, correctness assertions enabled, and limitations.
- Produce a per-task report under `docs/reports/` (referenced by `docs/PERFORMANCE.md`) and feed the consolidated result into CHRON-036.

## Out of Scope
- Relaxing the 3 GB / 5 GB / 7 GB caps or the 60 FPS / 100-person Phase 1 budget; any such change needs product-owner approval and a Change Proposal.
- Implementing new gameplay systems to improve a number; this is measurement only.
- NLG / history-query benchmarks that have no Phase 1 system yet; record as N/A/not-applicable rather than inventing a number.
- Full-ECS relationship/memory/ecology benchmarks (Phase 2+); mark as diagnostic N/A.
- Rendering beyond the snapshot/bridge baseline; CHRON-031 owns the 60 FPS presentation claim.
- Treating one noisy CI-run number as a performance gate (PERFORMANCE.md rule); reference-machine M5 local results only.

## Dependencies
- CHRON-028 complete (kernel/unit that is raced at scale).
- CHRON-029 complete (Render Snapshot DTO measurement input).
- CHRON-030 complete (worker command/snapshot latency applicable input).
- CHRON-031 complete (real Godot presentation and frame/bridge capture input; required for rendered comparison).
- CHRON-032 complete (10-year chaos report as the correctness baseline the benchmarks must not regress).
- CHRON-027 (action throughput), CHRON-026 (utility), and CHRON-024 (pathfinding) provide applicable per-system counters transitively through the kernel.

## Execution Steps / Readiness

1. Inventory each metric's production path, counter and instrument before the
   sweep; D4/§4 define the same-workload controls. Add missing adapters/counters
   within §3 rather than inventing metrics after the run.
2. Create `bench_micro_world`; smoke 100 Persons and verify action completions,
   actual pathfinding calls, queue bounds and deterministic digests.
3. Run scales sequentially with two warm-ups and at least ten timing samples;
   memory uses separate cold samples. Run direct/worker/rendered comparison
   with matching simulated work, not mismatched pacing or an old spike ratio.
4. Parent reviews raw samples and method; failed/unfinished scales remain
   visible. No hidden scale-specific shortcut, budget adjustment or gameplay
   optimization is authorized by this measurement Task.

## Files Modified / Allowed
- `benchmarks/**` and `apps/headless-runner/**` benchmark bins (mirroring `bench_10k_entities`, `bench_event_throughput`).
- `crates/sim-core/**` for any small benchmark-only diagnostic accessor (prefer no production API changes; add read-only counters).
- `docs/PERFORMANCE.md` (add task-specific raw-result references only; do NOT relax the budget lines or the caps).
- `docs/reports/CHRON-033_SCALE_BENCHMARKS.md` and any per-scale raw artifacts.
- `docs/tasks/CHRON-033.md`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- One entry, e.g. `bench_scale(persons: usize, seconds: SimDuration) -> ScaleResult`, where `ScaleResult` is a structured, serializable bag of per-metric medians with a `scale` label (100/1000/3000/5000/10000).
- Every metric includes: sample count, warm-up, median, min/max or variance, build profile (release), and the exact command/flags.
- The harness keeps correctness assertions enabled: an invariant violation fails the sample, and is reported rather than silently skipped.
- A metric that has no Phase 1 companion system is reported as `NotApplicable` (e.g., `nlg`, `history_query`, `relationship_full`) rather than fabricated.
- The 100-person result carries a documented Phase 1 contract verdict; higher scales are explicitly labeled diagnostic and must not be cited as a relaxed budget.

## Tests / Validation
- Fixture correctness: each scale's workload genuinely exercises the representative loop (needs, utility, movement, action) and respects the person count exactly.
- Warm-up/sample conventions: no single-sample claim; median over ≥10 post-warm-up samples per scale.
- Scaling sanity: report the actual RSS/throughput trend and investigate deviations; do not force monotonic results, discard samples or add hidden per-scale branches.
- Workspace gates: fmt, Clippy with warnings denied, release tests (bench binaries compile and run a smoke scale), docs, dependency audit.

## Benchmark
- Scale sweep 100/1K/3K/5K/10K on M5 16GB, release, ≥10 post-warm-up samples per scale, median + variance.
- Report per scale: peak RSS delta, sim-seconds-per-wall-second, throughput, scheduler enqueue/dequeue throughput and max depth, utility decisions/s, pathfinding cost (if present), event creation/s, and Render Snapshot build+serialize bytes/time.
- Record the 100-person result against the Phase 1 hard gate and the 3 GB / 5 GB / 7 GB caps; record any observation that may indicate the caps are at risk — but do not relax them.

## Definition of Done
- The 100/1K/3K/5K/10K scale sweep is measured on the M5 reference machine with the documented methodology and reported per system where applicable.
- The 100-person result is evaluated against the Phase 1 hard gate; no budget is relaxed.
- Higher scales are reported as diagnostics; they are never used to widen the 3/5/7 GB caps or the 60 FPS / 100-person budget.
- Un-implemented systems are reported as NotApplicable, not fabricated numbers.
- Results are reproducible and documented in `docs/PERFORMANCE.md` and a `docs/reports/` task report, and feed into CHRON-036.

## Required Completion Report
Report: change summary; commands run; the full scale-sweep table (RSS delta, throughput, scheduler, utility, pathfinding where present, events, snapshot/bridge) per scale; the 100-person Phase 1 gate verdict; any metric that hit N/A and why; known limitations (e.g., single-threaded kernel, no Phase 2+ systems, dummy-vs-real distance); and any observation that the caps may be at risk (without relaxing them). Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
