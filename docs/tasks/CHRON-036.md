# CHRON-036 — Phase 1 Validation Report

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Consolidate the verified Phase 1 results into `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md`: the M5 measured outcomes, the 100-person 10-year run, presentation FPS, headless-vs-rendered comparison, risks, recommendations for `bevy_ecs` and the Simulation worker, and the product-owner decisions required before Phase 2. This Task reports only; it does not enter Phase 2.

## Context
Phase 0 ended with `ARCHITECTURE_SPIKE_V1.md`, which confirmed Godot+Rust and provisionally continued `bevy_ecs` but explicitly required re-evaluation after representative components, Scheduler integration, snapshots, and rendered/headless comparison existed. Phase 1 is the "Micro World Kernel": World Grid, Terrain, Local Tile, Person Entity, Basic Movement, Time, Needs, Basic Utility AI, with a DoD of 100 NPCs that can move/eat/sleep/work continuously for 10 years without crashing. The product owner must now see one consolidated, evidence-backed report to decide whether to continue the `bevy_ecs` hypothesis (durable vs provisional), whether the in-process Simulation worker is the right model (per the ADR-0007/ADR-0010 direction), and whether Phase 2 is authorized. This Task is the Phase 1 analog of CHRON-014.

## Scope
- Synthesize the Phase 1 task results, benchmarks, ADRs, risks, memory-budget interpretation, technology/worker recommendations, and required product decisions into `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md`.
- Include the consolidated M5 16GB reference measurements: 100-person 10-year chaos run (CHRON-032), the 100/1K/3K/5K/10K scale sweep (CHRON-033), headless-vs-rendered comparison, scheduler/utility/pathfinding/events/snapshot where measured, and presentation FPS (CHRON-031).
- Answer, with evidence, the questions that determine the next phase:
  - Is `bevy_ecs` (0.19.1) a durable runtime-ECS choice for the real kernel, or should it be replaced? (Phase 0 left this provisional per ADR-0005.)
  - Is the in-process single Simulation worker (CHRON-030) sufficient, or is a process/IPC worker warranted? (ADR-0007/ADR-0010 deferred that choice.)
  - Do the real kernel measurements meet the Phase 1 hard gate (100-person/10-year correctness) and the 3/5/7 GB caps and 60 FPS target?
- Summarize memory-budget interpretation for Phase 2+, with explicit note that the 3/5/7 GB caps are retained unless the product owner explicitly changes them via a proposal.
- List the known risks (dummy-vs-real distance, history growth, single-threaded kernel, snapshot/save hardening, editor-exit crash monitoring) and the Phase 0 leftover warnings that remain.
- Clearly mark that this report is report-only and that **Phase 2 implementation must not begin** until the product owner explicitly approves it.
- Reference the task-specific raw reports for exact methods; the report must not invent or overstate measured data.

## Out of Scope
- Implementing any Phase 2+ system (Age, Birth, Death, Family, Personality, Values, Skills, Profession, Relations, Memory, economy, ecology, warfare, politics, religion, magic).
- Changing `MASTER_SPEC.md`; hiding benchmark limitations; treating prototypes as production formats.
- Relaxing or tightening any memory cap, the 100-person budget, or the 60 FPS target.
- Building new gameplay systems to improve a number; this Task reports only.
- Anything LLM/NLG/Rule-Editor/Web client-facing.
- Starting a new Phase-1 implementation Task (this is the closing report of Phase 1).

## Dependencies
- CHRON-031 complete (Godot presentation FPS report).
- CHRON-033 complete (scale sweep RSS/throughput).
- CHRON-034 complete (regression/CI gate status and any CI-derived evidence).
- CHRON-032 complete (100-person 10-year chaos result).
- CHRON-029, CHRON-030 (snapshot DTO and worker reports) for the worker/recommendation inputs.
- CHRON-028 (kernel numbers) feed the scale sweep and bevy recommendation.
- CHRON-035 (spike-workload retirement status) for the honesty/removal record.
- `ARCHITECTURE_SPIKE_V1.md`, `docs/PERFORMANCE.md`, all ADRs, and `docs/ARCHITECTURE.md` for the context the report must reference.

## Files Modified / Allowed
- `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md` (**new report**; the sole required deliverable).
- `docs/tasks/CHRON-036.md`.
- Reference to raw result artifacts only; do not reproduce full tables with invented values.
- No product document change; no `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` edits without a Change Proposal. ADRs are referenced, not rewritten.

## API Contract / Evidence
- The report must carry an explicit header: Phase (1 — Micro World Kernel), reference machine (Apple M5 10-core/16 GiB), OS, status (Complete & pending product-owner confirmation), and confirmation date placeholder.
- Every required question (bevy_ecs durability, worker/IPC sufficiency, Phase 1 gate, memory budget interpretation) is answered with a named measurement and a clear "continue / change / defer" verdict.
- Any statement that a measurement does not support must be labeled as such; no invented conclusion (mirrors CHRON-014's "no invented/unmeasured conclusions" rule).
- The report must explicitly state that Phase 2 is blocked pending product-owner approval and that the spike-workload retirement (if done) is historical-only.
- The report must reference `docs/PERFORMANCE.md` methodology and the task-specific reports for raw numbers.

## Tests / Validation
- All required fields present and audited against source reports: M5 machine, 100-person 10-year result, FPS, headless/rendered ratio, scale-sweep numbers, bevy + worker recommendations, risks, and the Phase 2 product decision.
- Cross-check: every number in the report matches its source report (CHRON-032/CHRON-033/CHRON-031); no fabricated metric; no mention of a system that does not exist.
- Clean-CI rerun: the Phase 1 CI gates (CHRON-034) and Rust/Godot local gates still pass at report time; no test skipped or weakened.
- The Master Spec hash guard is intact.
- If CHRON-035 retired the spike, confirm the report references the real-kernel numbers and not the retired dummy numbers as current.

## Benchmark
- No new workload; this report only consolidates measurements captured by bounded benchmark tasks (CHRON-031, CHRON-033, CHRON-032). It may present the authoritative M5 reference-suite results as the Phase 1 gate, but it does not generate new measurements.

## Definition of Done
- `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md` exists and answers every required question with evidence: M5 results, 100-person 10-year outcome, presentation FPS, headless-vs-rendered comparison, risks, recommendations for `bevy_ecs` and the Simulation worker, and the Phase 2 product decision.
- It explicitly marks evidence limits and does not invent or overstate measurements.
- It retains the 3/5/7 GB caps and the 100-person/60 FPS gate and does not relax them.
- It states clearly that Phase 2 implementation is blocked pending product-owner approval, and that the spike-workload retirement (if any) is historical-only.
- Clean CI/reference gates pass at report time; no test removed, skipped, or weakened.
- Phase 2 remains stopped until the product owner approves the next phase.

## Required Completion Report
Report: change summary; commands run; benchmark consolidated table or explicit N/A-per-source with pointer to each source report; the list of covered questions and their evidence; known limitations (e.g., single-threaded kernel, no Phase 2+ systems, snapshot/save hardness, editor-exit crash monitoring); the Phase 2 recommendation presented to the product owner; and any blocker. Do not start Phase 2 or any next Task; each requires separate product-owner approval.
