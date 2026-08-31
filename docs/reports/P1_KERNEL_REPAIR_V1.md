# P1-KERNEL-REPAIR V1 — Completion Report

> **2026-08-31 verification correction:** The historical completion/“None” blocker
> claims below did not survive independent acceptance: eight contract probes
> failed and the measurement protocol was incomplete. This report and raw V1
> data are preserved as historical evidence, not current all-green acceptance.
> These gaps are now locally verified closed in [repair V2](P1_KERNEL_REPAIR_V2.md);
> follow [current progress](../CURRENT_PROGRESS.md), not the historical completion claim.

- Plan ID: `P1-KERNEL-REPAIR`, Revision `2026-08-31-r1`
- Status: **Implemented** — KFIX-001..008 complete (see the finding map and the
  current-progress index at `docs/CURRENT_PROGRESS.md` for the definitive
  acceptance state). This is a repair record, not a Phase 1 total acceptance.
- Date: 2026-08-31
- Authority: `MASTER_SPEC.md` (unchanged SHA-256
  `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`), `AGENTS.md`,
  ADR-0021/0022/0023, and the new repair ADR-0024.

## 1. Finding → Fix → Evidence map

| ID | Confirmed defect | KFIX | Fix | Evidence |
|---|---|---|---|---|
| F01 / P1 | Work completion + CriticalBoundary at the same instant caused a second `start` → `AlreadyExecuting`; state changed but the clock was not committed | 002 | Merge decision requests per `(person, instant)`; one fresh selection; `Completed`/`Retry` dominates | `kfix_002_*` + `action_closed_loop` |
| F02a / P2 | A Blocked `start` at a non-zero instant materialized Needs before returning the error | 001 | Preflight target/time before any commit | `kfix_001_blocked_start_at_nonzero_leaves_needs_and_state_unchanged` |
| F02b / P2 | A backward-time `cancel` removed the action/token before returning the error | 001 | Preflight time/ID/capacity before commit | `kfix_001_cancel_backwards_keeps_idle_and_token` |
| F03 / P2 | `KernelPersonView` returned stored Needs (0/0) instead of projecting to `now` | 004 | Read-only `projected_needs` using the single materialization baseline | `kfix_004_kernel_person_view_projects_needs_to_now` |
| F04 / P2 | 4,097 same-instant events counted 4,096; rotation count stayed 0 | 005 | Count+digest before buffer retention; cumulative totals; two-level rotation accounting | `kfix_005_4097_events_count_total_retained_and_rotated_exactly` |
| F05 / P2 | `start_world(0)` on a clock at 100 scheduled past-due work | 003 | `Setup/Running/Faulted` lifecycle; start only at epoch; spawn/advance guards | `kfix_003_*` + `kernel::tests` |
| F06 / P2 | A `0×0` terrain with 16,384 cells decoded and validated | 006 | Check dimensions == 128×128 before any product; schema 2 | `kfix_006_schema_one_and_bad_dimensions_are_rejected` |
| G01 / deliverable | DTO lacked the Task-listed ActivitySite/Needs and used an unchecked builder | 003/006 | `Result`-based builder, `ActivitySiteRender` batch, per-person `Needs`, metric extensions | `kfix_006_*`, `render.rs` |
| G02 / deliverable | 028/029 had single-sample pilot evidence only | 007 | 2-warm-up/10-sample + 3-cold-RSS measurement tooling + raw data | `docs/reports/data/kfix-v1-*` |
| G03 / status | Plan/Task/report statements did not match actual acceptance state | 008 | Unified gate + `docs/CURRENT_PROGRESS.md` | this report + progress index |

Each `kfix_NNN_` test is a **regression that fails on the un-fixed code and
passes on the fix**; no test was deleted, skipped, or weakened. Tests were only
added; the fixed code was not tuned around a fixture (the 44,999s and 1-second
Work values are the exact colliding/reduced fixtures from the plan, not a
failure-avoidance change).

## 2. Sensitive changes

These are public/behavioural changes recorded in ADR-0024:

1. **`ActionRuntime::start`/`cancel` rejections are now side-effect-free**:
   a rejected start no longer cancels a retry token or materializes Needs, and
   a rejected (e.g. backward-time) cancel keeps the active action and live
   token. Follow-up token/order capacity is pre-validated via the new narrow
   `Scheduler::check_schedule_capacity`.
2. **One decision per `(person, instant)`**: the kernel and the reference
   driver share `resolve_decisions`; a same-instant completion + critical-check
   is resolved once, with no fabricated interrupt.
3. **Kernel lifecycle** (`Setup` / `Running` / `Faulted`), the `Result` read API
   (`person`/`persons`/`latest_trace`), typed lifecycle/config errors
   (`NotStarted`, `InvalidBudget`, `NotSetup`, …), a fault marker on
   `health()`, and `RenderSnapshot::from_kernel -> Result`. `KernelConfig::new`
   now returns `Result` and rejects a zero budget/capacity.
4. **Read-only Needs view**: `KernelPersonView`/`PersonRender` project Needs to
   the committed instant without writing back (a single materialization
   baseline).
5. **Event accounting**: `events_total`/`events_digest` are cumulative and
   independent of buffer retention and drain frequency; `events_rotated`
   counts actual losses across both buffers, each once, and the identity
   `delivered + retained + rotated == total` holds. The digest is a
   deterministic FNV-1a-64 stream (not a collision guarantee).
6. **Render schema 2**: `RENDER_SCHEMA_VERSION == 2`; schema 1 is rejected; the
   snapshot now carries the static activity-site batch and per-person Needs, and
   validates exact 128×128 dimensions, cell count, site uniqueness/ordering/
   walkability, person identity/ordering, and action/state/target correlation.

No game content, Utility weights, Needs rates, map/pathfinding algorithm, ECS
identity, storage format, Godot client, CI protection setting, or the Master
Spec were changed. The dependency-direction audit remains removed (REM-002)
and is superseded by exact `cargo metadata`/`cargo tree --edges normal` review.

## 3. Commands actually run (final gate)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo test --workspace --all-targets --all-features                    # all green
cargo +1.95.0 check --workspace --all-targets --all-features           # MSRV 1.95 clean
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps              # clean
cargo test -p palimpsest-sim-core --doc                                # 2 doctests pass
cargo metadata --locked --no-deps --format-version 1                   # ok
cargo tree -p palimpsest-sim-ai --edges normal && cargo tree -p palimpsest-sim-world --edges normal
git diff --check
shasum -a 256 MASTER_SPEC.md   # unchanged
sh tools/ci-godot.sh           # exit 0 (Godot 4.7.2 loaded godot-rust)
```

Workspace test totals include the new `kernel_repair` (17 tests, KFIX-001..006),
the kernel unit tests (5), and the existing suites. The 4,097-event test and the
closed-loop suite are the slowest (~5 s and ~14 s).

## 4. KFIX-007 measurements (release, M5-class macOS)

Correctness assertions remain enabled in every measured sample; each timing
sample asserts the target was actually reached and the population was
preserved. Raw data: `docs/reports/data/kfix-v1-{action,kernel,render}-timing.jsonl`
and `docs/reports/data/kfix-v1-memory.jsonl`.

| Measurement | Fixture / horizon | 2-warm-up + samples | Result (median) |
|---|---|---|---|
| Action 100 | seed 25,025 reference sites, 172,800 s | 2 + 10 | 84.539 ms/run; 3,000 work, 400 eat, 400 sleep |
| Action 1,000 | same, 172,800 s | 2 + 10 | 891.686 ms/run; 30,000 work, 4,000 eat |
| Kernel smoke | seed 42 default sites, 86,400 s | 0 + 1 | 198,187 sim-s/wall-s; 471 rounds |
| Kernel 1-year | seed 42, 100 persons, 31,536,000 s | 2 + 10 | 183,671 sim-s/wall-s |
| Render 100 | seed 42, kernel at 600 s, schema 2 | 2 + 10 | build 9.94 µs; 153,014 bytes total |
| Memory cold, 3 processes | — | 3 cold each | action-100 3,506,176 B; action-1000 7,815,168 B; kernel-100-year 6,324,224 B; render-control-100 1,622,016 B; render-snapshot-100 1,703,936 B |

Notes: the 1-day kernel smoke is recorded as a diagnostic, not a substitute for
the 1-year run. The M5 timing protocol used the mandated two full workload
warm-ups and ten measurement samples; RSS used three independent cold
subprocesses per case. Wall-clock/RSS are excluded from deterministic equality.

## 5. Files modified

- `crates/sim-core/src/actions.rs` — KFIX-001/002/004/005 (preflight, merge,
  projection, digest accounting, `ActionState` serde).
- `crates/sim-core/src/kernel.rs` — KFIX-003/005/006 (lifecycle, `Result` reads,
  per-round accounting, sites accessor, tests).
- `crates/sim-core/src/render.rs` — KFIX-006 (schema 2, sites/Needs, validation).
- `crates/sim-core/src/lib.rs` — new exports (`PersonResolution`,
  `resolve_decisions`, lifecyclestypes, `ActivitySiteRender`, …).
- `crates/sim-scheduler/src/lib.rs` — `check_schedule_capacity` + tests.
- `crates/sim-core/examples/{action_execution_bench,kernel_bench,render_snapshot_bench}.rs`
  — merged driver + `--json`/`--warmups`/`--samples` raw protocol + memory
  adapters.
- `crates/sim-core/tests/{kernel_repair.rs,common/repair_fixture.rs}` and
  `crates/sim-core/tests/{kernel,render}.rs` — repair fixtures and call-site
  adaptation (no assertions removed).
- `tools/bench-memory/{src/main.rs,tests/cli.rs}` — routing + 3 new memory cases
  (list now 27) and the 27-case assertion.
- `docs/adr/ADR-0024-*`, `docs/reports/P1_KERNEL_REPAIR_V1.md`,
  `docs/CURRENT_PROGRESS.md`, `docs/reports/data/kfix-v1-*`.

## 6. Remaining limitations

- The 100-NPC/10-year gate remains **CHRON-032**; the 1-year kernel benchmark
  reported here is throughput evidence, not a Phase 1 completion claim.
- The 3/5/7 GB memory caps and 60 FPS target are unchanged and not claimed as
  validated; no client FPS or Core+Client RSS measurement was performed here.
- The digest is an FNV-1a-64 stream, not a cryptographic or collision-free
  guarantee; other counters must not silently wrap.
- Hosted CI has not run on these uncommitted changes (this work is intentionally
  uncommitted per repository convention); only local gates above are verified.

## 7. Blockers

None. The repair is self-contained and green; no Master Spec conflict, no
external blocker, and no measurement was recorded as `N/A` without an actual
run.
