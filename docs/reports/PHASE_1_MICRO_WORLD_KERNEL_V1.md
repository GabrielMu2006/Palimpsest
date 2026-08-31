# Phase 1 — Micro World Kernel V1

Status: evidence assembly in progress; NOT yet complete or owner-confirmed.
Phase2 implementation is not authorized. Owner confirmation date: unset.
Reference platform: Apple M5,16GiB,macOS26.6.2,Rust1.98.0,Godot4.7.2;
locked bevy_ecs0.19.1. Candidate/hosted-check identities will be recorded after
publication; raw source manifests identify the measurements independently.

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

Corrected native windowed presentation:120warmup+300consecutive monotonic frame
intervals, mean59.975FPS,min47.714FPS,p95FPS61.072; p95frame16.997ms,max20.958ms.
This is approximately60Hz with observed jitter, not constant60FPS. Main UI draw
calls19; full-process high-water277954560B on this capture. Snapshot age p95102.169ms,
construction90us, bridgeconversion110us, fullsnapshotcall120us (all p95).
Actual MultiMesh transforms/colors were read back and a deliberate corruption
was detected. Headless CI explicitly cannot replace this GPU evidence.

## Representative scale and matching-work measurements

Pending completion of CHRON033. The final table must include100/1K/3K/5K/10K
attempts, all raw samples, native RSS proof, applicable work/cost metrics, and
same-horizon direct/worker/windowed comparison. Do not treat this placeholder as
a completed gate. The source [ADR0029](../adr/ADR-0029-representative-benchmark-observation.md)
fixes identical fixtures, warmups, sample policy, counter boundaries and limitations.

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

CI, spike retirement and final candidate evidence are pending; fill before delivery.
The owner will accept/reject this Phase1 report separately. Acceptance alone does
not invent a Phase2 execution plan. Do not begin Phase2 implementation without
explicit authorization of its scope.

Measurement boundary clarification: Core native high-water is read at the completed
workload boundary, before the outer CLI's final JSON output encoding. It is not a
promise about every later adapter allocation. Godot's `/usr/bin/time -l` result
covers the whole windowed process. Neither serialized bytes nor RSS increments
are mislabeled as a full configuration's total memory footprint.
