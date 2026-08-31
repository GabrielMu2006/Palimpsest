# CHRON-035 — Retire Phase 0 Spike Workload

> **Status: Authorized — current033–036 owner instruction; completion waits for verified dependencies.**
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Once `sim-core` has a real Phase 1 world kernel, retire the temporary Phase 0 "shared spike workload" (`run_spike_workload`) and its associated public API, headless spike binary, and Godot bridge spike benchmarks. Preserve the original Phase 0 baseline and reports for the record, update ADR-0010 status, and retarget CI to the real kernel benchmark. Do not delete the historical reports.

## Context
ADR-0010 states `sim-core::run_spike_workload` is "temporary measurement infrastructure, not a permanent gameplay API. It must be reviewed or removed before Phase 1 turns sim-core into a real world kernel." If a real kernel now exists, the spike workload serves no purpose. Worse, it manufactures a "we're fast" claim on a dummy loop (no needs, psychology, pathfinding, ecology, or history) that can entice the product owner into relaxing the 3/5/7 GB caps against a workload that will never translate to a real world. `ARCHITECTURE_SPIKE_V1.md` calls this the "dummy-workload optimism" risk. This Task executes that review-and-remove only after a real kernel, benchmark, and runner (CHRON-028, CHRON-032, CHRON-033) actually replace the spike's purpose; if the spike is worse than the real kernel on any axis, that delta is reported honestly and no budget is relaxed. The original Phase 0 baseline survives purely as an historical record.

## Scope
- Require, before removal is allowed, that a real Phase 1 kernel (CHRON-028), real benchmark (CHRON-033), and CHRON-034's regression gate (which depends on the real CHRON-032 chaos runner) are complete and recorded; ADR-0010's "review or remove" condition is met.
- Remove only `run_spike_workload`, its spike-only public API and direct obsolete callers/dependencies. No unrelated cleanup is permitted.
- Preserve, unchanged, the original Phase 0 reports, raw artifacts, and the headless/rendered comparison numbers (CHRON-008, CHRON-017, and ARCHITECTURE_SPIKE_V1) as historical record; only delete the source that implements the dummy workload and a direct public API re-export if the reuse would be misleading.
- Update ADR-0010's status/decision to record the spike workload is retired and the replacement rationale, citing the positive evidence of replacement; if the real kernel numbers were worse than spike on any axis, record that honestly and do NOT relax a budget.
- Update CI (CHRON-034) to remove any spike-workload benchmark/smoke step and to replace or tie the workload with the real kernel-driven benchmark from CHRON-033/CHRON-032; CI must not inherit a harmless dummy number.
- Remove/refresh any README, docs/reports mention that presents a dummy "we're fast" number; keep the honest historical delta where needed.
- Keep the workspace compiling with clippy/GDA-checked Rust gates passing, and the master spec hash guard intact.

## Out of Scope
- Deleting or rewriting the Phase 0 reports, raw artifacts, or CHRON-017/CHRON-008 numbers (per the hard requirement: do not delete the historical reports).
- Relaxing or lowering the 3/5/7 GB caps or the 100-person/60 FPS gate.
- Building a new "pretend fast" workload; if a real workload doesn't exist, the Task waits and the spike stays until the replacement is present.
- Implementing new gameplay systems or tuning the kernel to look faster on a dummy.
- Anything Godot/LLM/gameplay unrelated to removing this specific spike harness.

## Dependencies
- CHRON-028 complete (real kernel exists and replaces the spike's purpose).
- CHRON-033 complete (real scale benchmark replaces the spike baseline).
- CHRON-034 complete (CI/regression is on the real kernel and transitively requires the CHRON-032 chaos runner).

## Execution Steps / Readiness

1. Parent inventories `run_spike_workload`, `SpikeRunMetrics`, `SpikeRunError`,
   bridge method, GDScript probe, CLI and CI references before deletion.
2. Map each existing test/assertion and smoke to real-kernel replacement or
   retained primitive coverage. If equivalent coverage is absent, build it
   before retiring the API; never delete a failing test to complete removal.
3. Record ADR-0010 retirement evidence and snapshot historical report/artifact
   hashes from the current worktree (not old HEAD, which predates remediation).
4. Remove only the reviewed direct path, validate replacement across Rust and
   Godot, update the same candidate PR/CI per P1-REMAINING and compare hashes.
   Parent owns this cross-module removal; no open-ended cleanup delegation.

## Files Modified / Allowed
- `crates/sim-core/**` (remove the spike workload module and its re-exports; update the crate's `lib.rs` and its test harness if `run_spike_workload` is referenced).
- `crates/godot-bridge/**` (remove the `benchmark_spike_workload` bridge method and its presentation dictionary if it only existed to exercise the spike; keep the render snapshot / micro world path).
- `apps/headless-runner/**` (remove the spike-based CLI path / binary if the spike workload is referenced; keep the real kernel runner).
- `.github/workflows/ci.yml`, `tools/ci-rust.sh`, directly calling Godot scripts/benchmark tools, `README.md` and `docs/PERFORMANCE.md` current-result references; historical reports/raw artifacts stay unchanged.
- `docs/adr/ADR-0010-shared-spike-workload.md` (update status/replacement evidence).
- `docs/tasks/CHRON-035.md`.
- `docs/reports/CHRON-035_SPIKE_RETIREMENT.md` records the removal/coverage map and hashes.
- Do NOT modify `MASTER_SPEC.md`; do NOT delete `docs/reports/CHRON-008_10K_DUMMY_BENCHMARK.md`, `docs/reports/CHRON-017_HEADLESS_RENDERED.md`, or `docs/reports/ARCHITECTURE_SPIKE_V1.md`; these stay for the record.

## API Contract
- After removal, `sim-core` exposes no `run_spike_workload`, `SpikeRunMetrics`, or `SpikeRunError`; the headless runner and godot-bridge no longer reference the spike function or a spike-specific benchmark method.
- The documented replacement is the real kernel-driven path: the headless runner and the Chaos runner both call the Phase 1 kernel; the Godot client consumes `RenderSnapshot` (CHRON-029) via the worker (CHRON-030), never the spike.
- Historical reports and explicit plan/retirement records may still name the spike. No current executable export/caller or current-performance claim may depend on it.
- Any public API that would re-export the dummy for a reuse is removed; no "temporary" spike code may remain to be silently reused.
- CI gates run compile/tests/regression/chaos-smoke (real kernel) and no spike-workload benchmark step.

## Tests / Validation
- Removal test: no executable caller/export remains; a compile-fail doctest or separate negative fixture proves the retired API is unavailable. Do not leave an intentionally uncompilable normal workspace test. Preserve equivalent existing assertion coverage on the real path.
- Replacement test: the headless runner, Chaos runner, and Godot snapshot path run on the real kernel and/or snapshot (not the spike) after the removal; the 10-year CHRON-032 report still passes where the CHRON-028 kernel is the driver.
- Historical-record test: `ARCHITECTURE_SPIKE_V1.md`, `CHRON-008_10K_DUMMY_BENCHMARK.md`, `CHRON-017_HEADLESS_RENDERED.md` are still present and unmodified; the Master Spec hash is unchanged.
- CI test: the spike benchmark/smoke is removed from CI; the CI still passes with the replacement/snapshot path, and no "dummy fast" number is reported as current in performance docs.
- Workspace gates: fmt, Clippy with warnings denied, debug/release workspace tests, docs, dependency audit, and the master-spec hash guard.

## Benchmark
- No new benchmark to establish; the spike workload's benchmark is retired.
- The real numbers from CHRON-032/CHRON-033 stand; if the removal causes the real kernel benchmark to be the only number going forward, note that transition in the report. Any delta between the old spike and the current real measurement must be reported honestly as the real world's cost, and must not trigger a budget relaxation.

## Definition of Done
- The `run_spike_workload` (and its associated public types/APIs, headless spike binary, and godot-bridge spike benchmark) is removed from the Repository, and the Phase 1 kernel/runner/snapshot path is verified as the replacement and still works.
- ADR-0010 is updated to retired, with evidence that a real kernel/benchmark/runner replaced the spike and that this is not a reduction of the requirement.
- CI no longer references or runs the spike-workload benchmark, and the real kernel benchmark/regression runs.
- The historical Phase 0 reports (CHRON-008, CHRON-017, ARCHITECTURE_SPIKE_V1) remain intact and are not modified.
- No performance budget is relaxed and any real-vs-spike delta is documented honestly.
- The workspace compiles; rustfmt/Clippy passes; tests pass; master-spec hash guard intact.

## Required Completion Report
Report: change summary; commands run; the list of spike-workload files/APIs removed and the headless-runner/godot-bridge/CI paths that were retargeted; the status update written to ADR-0010; the confirmation that the historical reports are intact and unmodified; any measured real-vs-spike delta (and that no budget was relaxed); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
