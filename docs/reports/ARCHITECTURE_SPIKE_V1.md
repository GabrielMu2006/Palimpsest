# Palimpsest Architecture Spike V1

- Phase: **0 — Architecture Spike**
- Date: 2026-08-29
- Reference machine: Apple M5 MacBook Air, 10 cores, 16 GiB unified memory
- OS: macOS 26.6.2, arm64
- Status: **Implementation and local validation complete; awaiting product-owner confirmation**

`MASTER_SPEC.md` remains the read-only authority. Its verified SHA-256 is
`a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`.
This report does not authorize Phase 1.

## Executive Decision

**Recommend continuing with Godot 4 + Rust for Phase 1.** The Rust Simulation
Core runs fully without Godot, Godot consumes Rust-owned presentation data
through a narrow GDExtension adapter, the bridge is fast enough for batched
Render Snapshots, and the 128×128 renderer sustains the 60 FPS target.

**Recommend continuing the standalone `bevy_ecs` hypothesis provisionally.**
The 10K dummy benchmark is comfortably inside Phase 0 time and memory limits,
and stable `EntityId` remains separate from runtime ECS handles. This is not a
permanent commitment: movement, relationships, events, saves, and future LOD
must be benchmarked before the choice becomes durable.

No observed result requires a Change Proposal or architecture replacement now.
Several prototypes need hardening before production use, and one editor-only
Godot crash remains a tracked risk.

## Required Results

| Question | M5 16 GB result | Conclusion |
| --- | ---: | --- |
| 10K Dummy Entity RAM | **2.30 MiB RSS delta**, about **241 bytes/entity**; process RSS 4.18 MiB | Pass for simple two-component dummy entities; not representative of full NPCs |
| 10K Dummy simulation throughput | **1.270 billion simple component updates/s**; 10M updates in 7.873 ms median | `bevy_ecs` is not rejected by Phase 0 |
| Structured Event throughput | **36.409M validated events/s** generation; **6.239M events/s** JSON serialization | In-memory schema overhead is acceptable for the prototype |
| SQLite writes | **835,593 events/s** at 1,000-event batches; 849,984/s at 10,000; 76,883/s unbatched | Continue SQLite WAL; batching/backpressure are mandatory |
| SQLite size | **286.92 bytes/event** after checkpoint for compact dummy events | Real causal metadata will be larger |
| Snapshot size | **46,702 bytes stored** for 10K entities + 10K pending work; raw 248,259 bytes | Prototype compression is effective |
| Snapshot time | **0.964 ms encode**, **0.863 ms decode+validate** medians | Pass for dummy state; migrations and limits remain |
| Rust ↔ Godot scalar call | **354.67 ns/call** median, about 2.82M calls/s | Efficient enough, but production data must remain batched |
| 128×128 Tile rendering | **60 FPS** min/mean/p95 over 300 frames; **1 draw call**; 34.67 MiB video memory | Meets the normal-UI 60 FPS target under the spike workload |
| Headless / Rendered difference | **1.402 ms vs 2.928 ms** for identical 10K shared workload; Headless **2.09× faster** | Keep historical simulation independent of rendering |

Detailed methods and limitations are in the individual reports under
`docs/reports/`.

## M5 16 GB Memory Budget

The hardware exposes exactly 17,179,869,184 bytes (16 GiB) of unified memory.
During the final validation sample, macOS reported 52% system-wide free memory,
zero swap-ins, and zero swap-outs. Available memory on macOS is pressure-driven
and changes with compression, GPU use, and other applications; there is no
honest fixed “all RAM available to the game” number.

The spike supports **retaining, not relaxing**, the Master Spec's provisional
caps:

- MVP Core + Client: target **< 3 GB RSS**;
- 10K Simulation Core/Client: target **< 5 GB**;
- whole environment with optional Tiny LLM: target **< 7 GB**.

The 7 GB whole-environment ceiling leaves roughly 9 GiB nominal unified-memory
headroom for macOS, GPU allocations, compression variability, and development
tools. Phase 0's measured dummy Core and client baselines are far below these
caps, but they do not include real NPC state, pathfinding, history growth, final
UI assets, or an LLM. Therefore the budgets cannot yet be reduced or declared
fully validated; each Master Spec scale gate still requires measurement.

## Architecture Findings

The implemented dependency direction is:

```text
Godot client (rendering / input / read-only metrics)
                     |
        presentation-only GDExtension
                     |
      headless Rust Simulation Core
                     |
 structured events / snapshots / SQLite
```

- Godot Scene Tree state is presentation state, not simulation truth.
- `EntityId` is a nonzero monotonic `u64`, persisted independently of
  `bevy_ecs::Entity` runtime handles.
- `SimClock` uses deterministic signed integer seconds and is independent of
  wall-clock time and frames.
- Scheduler ordering is due-time then FIFO, supports cancellation/rescheduling,
  and exposes queue-health metrics.
- Structured events are versioned causal records; prose and historiography are
  not event truth.
- SQLite persists stable event representations transactionally; snapshots omit
  ECS handles and Scheduler heap internals.
- LLM, NLG, warfare, politics, religion, magic, Rule Editor, Web client, and
  full NPC AI remain absent from Phase 0.

## Phase 0 Task Result

| Task | Result |
| --- | --- |
| CHRON-001 Rust Workspace | Complete |
| CHRON-002 Godot 4 macOS Project | Complete |
| CHRON-003 Godot-Rust Bridge | Complete |
| CHRON-004 Stable EntityId | Complete |
| CHRON-005 Simulation Clock | Complete |
| CHRON-006 Scheduler | Complete |
| CHRON-007 Headless Runner | Complete |
| CHRON-008 10K Dummy Benchmark | Complete |
| CHRON-009 Structured Event | Complete |
| CHRON-010 Developer Metrics Overlay | Complete |
| CHRON-011 128×128 Tile Renderer | Complete |
| CHRON-012 Snapshot Prototype | Complete |
| CHRON-013 SQLite Event Store Prototype | Complete |
| CHRON-015 Test/Lint/Benchmark/CI | Complete locally; hosted run awaits remote push |
| CHRON-016 Event Throughput | Complete |
| CHRON-017 Headless/Rendered Comparison | Complete |
| CHRON-014 Architecture Spike Report | Complete; awaiting confirmation |

## ADRs Established

1. ADR-0001 — Rust Workspace Boundaries
2. ADR-0002 — Stable Entity Identity
3. ADR-0003 — Simulation Time
4. ADR-0004 — Scheduler Contract
5. ADR-0005 — `bevy_ecs` Spike
6. ADR-0006 — Structured Event Schema
7. ADR-0007 — Godot GDExtension Boundary
8. ADR-0008 — SQLite Event Store
9. ADR-0009 — Snapshot Format
10. ADR-0010 — Shared Phase 0 Mode-Comparison Workload

## CI and Quality Status

Local final gates cover rustfmt, Clippy with warnings denied, all workspace
tests/targets/features, Rust 1.95 MSRV, seven benchmark families including the shared mode
probe, Godot GDExtension initialization, and a full scene smoke run. The
`MASTER_SPEC.md` hash is a CI guard. No test was removed, skipped, or weakened.

GitHub Actions defines Ubuntu Rust quality/benchmark smoke and arm64 macOS Godot
integration jobs. The directory only gained Git metadata during Phase 0; it has
no selected remote, so no hosted workflow run can yet be cited.

## Known Risks

1. **Dummy-workload optimism.** Tight component loops and compact events do not
   model psychology, pathfinding, ecology, relationships, or long history.
2. **History growth.** Event retention, indexes, queries, compaction, and replay
   costs are not yet measured across 200 years.
3. **SQLite durability choice.** WAL + `synchronous=NORMAL` is fast but is not
   equivalent to `FULL`; crash-loss policy needs an explicit product decision.
4. **Snapshot hardening.** The prototype lacks migration tooling, decompression
   size caps, deltas, content-version compatibility, and hostile-file defenses.
5. **Godot editor crash.** One `--headless --editor --quit` invocation crashed
   after successful extension registration. Normal headless and windowed game
   paths remained stable. Recurrence outside the editor path is a stop signal.
6. **Main-thread bridge use.** The mode probe is synchronous. Phase 1 must decide
   how Simulation execution and Render Snapshot publication avoid frame stalls.
7. **Rendered benchmark variance.** Process-mode medians varied; 2.09× is a
   spike indicator, not a regression threshold.
8. **`bevy_ecs` evolution/MSRV.** Version 0.19.1 requires Rust 1.95 and remains a
   replaceable runtime implementation behind stable domain identity.
9. **Renderer simplicity.** The 60 FPS result uses one static generated atlas,
   no animation, path overlays, sprites, production panels, or camera effects.
10. **Hosted CI absent.** Local gates pass, but platform permissions, runner
    availability, and workflow behavior still need the first remote run.
11. **Long-run stability unproven.** Phase 0 does not claim a 200-year world,
    memory-leak freedom, or interesting historical emergence.

## Product-Owner Decisions Required Before Phase 1

1. Confirm the recommendation to continue **Godot 4.7 + Rust/GDExtension** with
   batched, presentation-only Render Snapshots.
2. Confirm `bevy_ecs` 0.19.1 as the provisional Phase 1 runtime ECS, with stable
   `EntityId` remaining the only persistent identity.
3. Confirm the existing **3 GB / 5 GB / 7 GB** memory caps remain unchanged for
   the next scale gates.
4. Select the Event Store durability policy: when `NORMAL` is acceptable and
   when a stronger checkpoint/`FULL` guarantee is required.
5. Decide the Snapshot compatibility promise for Phase 1: migration window,
   maximum decoded size, and whether old spike saves are intentionally invalid.
6. Select the Phase 1 Simulation/Render scheduling policy: synchronous budget,
   worker thread, or another snapshot-publication mechanism to spike.
7. Decide whether the isolated Godot editor-exit crash requires an upstream
   investigation before Phase 1 or may remain a monitored risk.
8. Select the GitHub remote and enable the first hosted CI run.
9. Review and explicitly confirm this report. **Until that confirmation, Phase
   1 must not begin.**

## Final Recommendation

Phase 0 has demonstrated the intended separation of concerns and has not found a
reason to abandon Godot + Rust or standalone `bevy_ecs`. Proceeding to Phase 1
is recommended **only after** the product owner confirms the decisions above.
The implementation remains stopped at the Phase 0 boundary.
