# Palimpsest — Current Progress

Updated: 2026-08-31. This is an execution index, not product authority or a new
implementation approval. Read the four mandatory specifications and relevant
current contracts; use [TASK_INDEX](TASK_INDEX.md) for historical routing.

## Active closeout review — 2026-08-31

The owner explicitly authorized repairing the review findings with Codex Luna
and completing CHRON-033–036. [ADR-0028](adr/ADR-0028-phase-1-review-closeout.md)
records the current contract and reduced-repeat RSS method. Earlier “033+
not authorized” statements below describe the previous handoff only.

030–032 review repairs are locally verified, including one corrected full ten-year
chaos/native-RSS run. [Review closeout](reports/CHRON-030_032_REVIEW_CLOSEOUT.md)
contains the findings and corrected evidence. CHRON-033 formal sequential sampling
is running; 034 regression/CI tooling is being prepared. No Phase2 work is authorized.
Earlier handoff entries below are historical context, not current authorization.

## Current outcome / active boundary

- Last completed: **CHRON-032 (Headless 10-Year Chaos Runner)**, locally verified
  2026-08-31. Evidence: [CHRON-032 report](reports/CHRON-032_CHAOS_10YEAR.md),
  [ADR-0027](adr/ADR-0027-phase-1-headless-10-year-chaos-runner.md), raw
  [timing](reports/data/chron-032-chaos.json) and
  [memory](reports/data/chron-032-memory.jsonl). Run: seed 42, 100 persons,
  315,360,000 s, 3 deterministic runs; min/median/max wall
  1627.4 / 1677.7 / 1685.9 s, ~188k sim-seconds-per-wall-second, 100 persons
  preserved, zero invariants, bounded scheduler queue (max 200 = 2/person).
  Phase 1 10-year no-crash gate met.
- Previous: **CHRON-031 (Godot Micro World Presentation)**, locally verified
  2026-08-31. Evidence: [CHRON-031 report](reports/CHRON-031_GODOT_MICRO_WORLD.md),
  [ADR-0026](adr/ADR-0026-phase-1-godot-presentation-contract.md), frame
  captures [full](reports/data/chron-031-frames.json) /
  [terrain-only](reports/data/chron-031-frames-minimal.json): 60/60/60 FPS,
  19 whole-scene vs 1 base-terrain draw call, zero runtime errors.
- Previous: **CHRON-030 (Simulation Worker / Command Bridge)**, locally
  verified 2026-08-31. Evidence: [CHRON-030 report](reports/CHRON-030_SIMULATION_WORKER.md),
  [ADR-0015 Phase 1 supplement](adr/ADR-0015-simulation-worker-command-render-snapshot.md),
  raw [timing](reports/data/chron-030-worker-bench.json) and
  [memory](reports/data/chron-030-worker-memory.jsonl) data.
- The CHRON-030 handoff's Luna subagent dispatch is unavailable in the Kimi
  Code runtime; the main agent implemented and verified 030/031 directly
  (fallback allowed by the dispatch skill).
- Previous: [P1-KERNEL-REPAIR-V2](tasks/P1_KERNEL_REPAIR_V2.md) locally
  verified 2026-08-31; [V2 report](reports/P1_KERNEL_REPAIR_V2.md) and
  [ADR-0025](adr/ADR-0025-kernel-repair-completion.md).
- V1 is superseded, not independently verified complete. Its eight additional
  failed contract probes and measurement gaps are addressed by V2; historical
  plans, reports and raw data remain available through the index.

## Phase / worktree

- Phase0: complete, owner confirmed 2026-08-29.
- CHRON-018–026 / REM-008A: historically delivered; see index for evidence.
- CHRON-027–029: implemented and locally verified through V2; see report.
- CHRON-030/031: implemented and locally verified 2026-08-31; see reports.
- CHRON-032: implemented and locally verified 2026-08-31; see report.
- CHRON-033–036: explicitly authorized by the current owner instruction; executing in dependency order. Phase2+ excluded.
- Branch `phase-1-planning`, HEAD `e5b0aeb676372a123dd8c27190e94b6a606d498c`.
  Existing implementation and this task are dirty/partially untracked.
  Local working-tree evidence is not a claim that remote HEAD contains it.
  No commit/push/PR or repository-setting change was made.
- MASTER_SPEC SHA256 unchanged:
  `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`.

## Contracts to carry forward

- Rust owns simulation truth; stable EntityId is distinct from ECS handles.
- start/cancel rejection is side-effect-free; each person's successful action
  commit sets a monotonic time watermark separate from lazy Needs accounting.
- Kernel Setup/Running/Faulted: dynamic person/trace/sites/next_due reads are
  Result-based; metrics label fault state and retain the last complete boundary.
  No rollback or recovery is implemented. Needs projection cannot silently
  fall back to an older value.
- Per-call generated event totals include retention losses. Cumulative digest
  and `total = delivered + buffered + rotated` are independent of draining.
- Render schema2: independent and root DTO decoding validates the same
  invariants; Moving Idle is invalid. Snapshot construction is fallible.
- Work budget1024 means due-instant rounds, not items or guaranteed wall time.
- RSS is proven native resident peak increment, not heap allocation bytes.
  The restored colocated Kernel fixture is not comparable to V1 BFS layout.
- The CHRON-030 worker owns the kernel on one dedicated thread; commands apply
  and snapshots publish only at complete committed boundaries; publication
  precedes acknowledgement; queue64/ack1024/step1000 bounds; speed changes
  pacing only; shutdown has an independent stop path that works when full.
- CHRON-031 Godot: one batched snapshot_frame per rendered frame; EntityId is
  8-byte LE on the wire (never f64/i64); enqueue failure vs applied/rejected
  ack are distinct UI states; Scene Tree holds presentation mirrors only;
  unavailable metrics are labelled, never fabricated.

## Evidence / remaining limits

CHRON-031 local gates: ci-rust.sh (fmt, deny-warnings clippy, workspace tests,
MSRV1.95, seven smokes), release workspace tests, doctests, rustdoc
-Dwarnings, metadata/tree review, bench-memory CLI, ci-godot.sh, gda
script/scene validation, 34-assertion headless integration, and two windowed
frame captures — all pass. 60 FPS is vsync-capped sustainment on M5, not
uncapped headroom.

Hosted required checks have not run on this dirty tree. No ten-year/100-NPC
gate or full Core+Client3/5/7GB budget acceptance is claimed.

Current authorized boundary: CHRON-032 is complete (the Phase 1 10-year
no-crash gate); 033+ remain later tasks and were not authorized by the 032
instruction. This index records completed evidence, not a blanket approval of
the remaining phase.
