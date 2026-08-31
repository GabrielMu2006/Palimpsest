# Phase 1 — Micro World Kernel V1

Status: implementation/report record; product-owner confirmation is unset.
Delivery acceptance requires both checks at the exact PR head; the final SHA and
hosted run are recorded in the [PR2 delivery record](https://github.com/GabrielMu2006/Palimpsest/pull/2).
A pending/failing check means delivery is not complete.
Phase2 implementation is not authorized. Owner confirmation date: unset.
Reference platform: Apple M5,16GiB,macOS26.6.2,Rust1.98.0,Godot4.7.2;
locked bevy_ecs0.19.1. Code source: [2d050b82138d92a6c2caf657f086123ab2d14441](https://github.com/GabrielMu2006/Palimpsest/commit/2d050b82138d92a6c2caf657f086123ab2d14441).
The delivery head adds reporting only; its literal SHA and CI run are recorded in
PR2 after verification, avoiding a self-referential report commit ID. Raw source
manifests separately identify each reference measurement.

## What exists

Rust owns the authoritative world and runs headlessly: World Grid, terrain/local
tiles, stable Person identities, deterministic simulation time, needs, pathfinding,
movement, basic utility decisions and Move/Eat/Sleep/Work/Idle execution. Structured
outcomes are validated/countable, with bounded rolling diagnostics. One worker
owns the kernel; Godot renders immutable snapshots, forwards time-control commands,
and shows population/actions/needs/queue/event metrics. Scene Tree changes never
become simulation truth. This is a micro-world demonstrator, not the full game.

Birth/death/family/personality/values/skills/professions/relationships, persistent
individual memories, economy/ecology/war/religion/magic/historians/NLG/LLM/RuleEditor
and a web client are not implemented here. Optional systems are NotApplicable in
these benchmarks; missing applicable measurements are never labeled that way.

## Correctness and presentation evidence

[030–032 review closeout](CHRON-030_032_REVIEW_CLOSEOUT.md) records the worker
ordering/interruption/publication fixes, real movement accounting, target-reference
validation, full deterministic comparisons and watchdog failure tests.

Corrected ten-year run:100 persons,seed42,315360000seconds/3650days, all100 completed
actual movement,Eat,Sleep,Work, no reported invariant violations. Wall1565.935157375s,
next_due315361289. Counts6717900events/decisions,202290027transitions,7934674rounds
and observed stream digest match historical outcomes. Movement phases2239300 are
now actual arrivals, not mislabeled activity completions. Full deterministic
schema2 report hash6301723614086087630; older hashes use an older contract.

RSS: one corrected native cold run, peak6619136B, baseline1540096B, proven
increment5079040B; prepared increment3325952B.3650 daily current-RSS observations
are retained. n=1 proves this observation only, with no sampling variance or
universal leak-free claim. The owner's original one-RSS sample remains valid for
its older colocated fixture; it is not substituted for this chaos fixture.

Native windowed presentation uses120warmup+300consecutive monotonic frame intervals.
The repaired capture measured mean59.975FPS,p95frame16.997ms. A single short capture
after the CHRON033 observers, on candidate8dc1595, measured mean60.002FPS,
min38.974FPS,p95FPS61.237; p95frame17.008ms,max25.658ms. Both raw captures remain
visible. This is approximately60Hz with observed jitter, not constant60FPS.
Latest normal-UI draw calls19; whole-process high-water278315008B. Snapshot age
p95102.451ms, construction90µs, bridgeconversion125µs, fullsnapshotcall139µs.
[Latest raw/source](data/chron-031-final-frame-source.json) identifies the code;
subsequent spike retirement does not change this normal presentation path.
Actual MultiMesh transforms/colors were read back and a deliberate corruption
was detected. Headless CI explicitly cannot replace this GPU evidence.

## Representative scale and matching-work measurements

[CHRON033 scale report](CHRON-033_SCALE_BENCHMARKS.md) contains every raw sample,
source/binary hash, work counter, isolated path probe and memory-proof interval.
All scales use the same seed42 reachable fixture and86400-second horizon, with
2warmups/10timed runs and one native cold-RSS observation per scale.

| Persons | Advance median s | Native workload peak B | Cold increment B |
|---:|---:|---:|---:|
| 100 | 0.430765 | 6422528 | 4866048 |
| 1000 | 4.306776 | 14843904 | 13287424 |
| 3000 | 13.040591 | 27377664 | 25821184 |
| 5000 | 22.128023 | 38420480 | 36864000 |
| 10000 | 44.261398 | 64454656 | 62898176 |

All60runs passed required work/state checks and within-scale deterministic
comparisons. Each RSS run's snapshot hash matches its timing fixture. These are
headless Core diagnostics, not full10K-game/client guarantees. Integrated scheduler,
decision, path-query, event and transition rates plus snapshot bytes/build/serialization
cost are in the task report, with method limits. No synthetic per-system cost is
presented where only an integrated work count was measured.

The matching100-person comparison uses direct median428.760ms, worker observed-ack
430.561ms and native Godot observed-ack563.488ms; ratios of medians are1.0042 and1.3142.
Godot's final target frame appears by median596.728ms from submission. Every run
has final DTO/work diagnostic hash14346005809762790435. Confirmation is observed
with1ms worker polling or a rendered frame in Godot, so those delays are explicit.
Godot whole-process high-water286244864B includes Core+Client. The Phase0 dummy
ratio~2.09 is not reused as a current result.

## Architecture recommendations

Continue bevy_ecs provisionally. The real100-person ten-year correctness result
and representative kernel measurements replace the Phase0 dummy as the decision
input. They provide no reason to replace ECS now, but do not establish permanent
suitability for unimplemented lifecycle/relationship/history/save workloads.
Revisit at the corresponding future approved milestones rather than rewrite now.

Continue the single in-process Simulation worker for the current100-person world.
Command ordering, pause/shutdown interruption and coherent publication boundaries
are tested; current windowed presentation consumes it without running truth on the
main thread. No measurement here demonstrates a need for process/IPC complexity.
This is not crash isolation from native Godot faults, nor a bound on every future
kernel operation. Scheduler round budgets are not wall-clock latency guarantees.

Retain all3GB(MVP Core+Client),5GB(10K) and7GB(optional TinyLLM) caps unchanged.
The measured100-person client observation is below3GB. Headless10K RSS alone
cannot certify a10K client; optionalLLM is absent so the7GB configuration is N/A.
Approximately60Hz presentation has the jitter disclosed above; no target is relaxed.
The200-year broader MVP ambition is not claimed from this Phase1 ten-year test.

## Risks and remaining decisions

- Rolling4096-event diagnostics are not durable EventStore history. History growth,
  retention and query performance remain future contracts, not deleted boundaries.
- Runtime snapshots are transient render DTOs, not hardened production saves;
  ADR0016 persistence hardening remains relevant.
- Single-threaded authoritative execution, future system cost and entity lifecycle
  need later measurements. Do not extrapolate current roughly linear results to
  all future systems or use an isolated path probe as an integrated CPU profile.
- Native RSS n=1 and short frame windows have explicit scope. Historical timings
  with different fixtures/methods are not interchangeable current numbers.
- Editor-exit crash monitoring remains; CI must report actual engine failure.
- The pre-existing custom dependency test was removed under explicitREM002 approval;
  locked metadata/tree receives manual review, not equivalent automatic enforcement.

## DoD and source routing

| Task | Outcome and evidence | Limits / final gate |
|---|---|---|
| 027 | [Action execution](CHRON-027_ACTION_STATE_MACHINE.md), committed-observation/closed-loop tests | Real arrivals, not zero-distance completion proxies |
| 028 | [Kernel](CHRON-028_KERNEL.md), full chaos and fixed-seed regression | Headless authority; bounded rounds are not latency guarantees |
| 029 | [Render DTO](CHRON-029_RENDER_SNAPSHOT.md), native fidelity test, same-work snapshot hash | Transient DTO, not a production save |
| 030 | [Review closeout](CHRON-030_032_REVIEW_CLOSEOUT.md), worker tests and033 comparator | Ordering/interruption/paired publication; no IPC claim beyond current scope |
| 031 | [Presentation](CHRON-031_GODOT_MICRO_WORLD.md), both corrected frame captures | Around60Hz with raw slow frames retained |
| 032 | [Corrected ten-year evidence](CHRON-030_032_REVIEW_CLOSEOUT.md) | One corrected long RSS/timing run; old timings retain their own source |
| 033 | [Scale report](CHRON-033_SCALE_BENCHMARKS.md) | All five scales passed; higher client/worker configurations not measured |
| 034 | [Regression/CI](CHRON-034_REGRESSION_CI.md), [hosted baseline](data/chron-034-hosted.json) | Both checks passed8dc1595; final delivery checks must match PR head |
| 035 | [Spike retirement](CHRON-035_SPIKE_RETIREMENT.md), negative API and replacement tests | Primitive diagnostics/historical reports remain; no dummy current-path alias |
| 036 | This report plus PR2's exact-head delivery record | No new workload, no Phase2 implementation |

Final local validation passed at the code source above:399executions in each
debug/release profile plus16doctests,814total, zero failures/ignored.
[Validation and complete logs](data/chron-035-036-local-validation.json) include
fmt/Clippy/MSRV/rustdoc, real/primitive smokes and Godot integration.
The publication record in PR2 names the final documentation head and both hosted
check conclusions, and links its immutable run. That external status must not be
inferred merely from local success or from the earlier034 green commit.

The owner will accept/reject this Phase1 report separately. Acceptance alone does
not invent a Phase2 execution plan. Do not begin Phase2 implementation without
explicit authorization of its scope.

Measurement boundary clarification: Core native high-water is read at the completed
workload boundary, before the outer CLI's final JSON output encoding. It is not a
promise about every later adapter allocation. Godot's `/usr/bin/time -l` result
covers the whole windowed process. Neither serialized bytes nor RSS increments
are mislabeled as a full configuration's total memory footprint.
