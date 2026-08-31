# REM-008A — Isolated Peak-Incremental RSS Measurement

- Status: Complete — parent independently verified, 2026-08-30.
- Authorization: the owner answered “纳入吧” to adding benchmark-tool changes
  beyond REM-008's original report-only whitelist. No repeated approval needed.

## Context

REM-008 delivered correctness gates and warm timing baselines, but `ps`
before/after retained deltas miss transient allocations. Whole-command peaks
mix seeds/scales/setup. Close that measurement gap without changing simulation.

## Scope

- Add one outward-only workspace benchmark tool, `palimpsest-bench-memory`.
- Read the macOS kernel RSS current/high-water counters in bytes; distinguish
  cold end-to-end workload and prepared-operation intervals (ADR-0020).
- Reuse the eight existing example workloads through additive memory adapters.
- Run each seed/scale/query in a fresh sequential subprocess, with three cold
  memory samples per case. Keep existing two-warmup/ten-sample timings intact.
- Reject ambiguous interval peaks, unsupported platforms and failed workloads;
  never relabel endpoints, allocator bytes or physical footprint as peak RSS.
- Verify transient allocation/release, prior-peak contamination, isolation,
  CLI failures, numerical invariants and deterministic workload checksums.
- Update reports and raw evidence with exact scope, units and limitations.

## Out of Scope

Production `src/` behavior/API changes, budget relaxation, optimization, Godot,
new game features, CHRON-027+, remote settings, commit/push/merge, OpenCode,
external model APIs, modifying MASTER_SPEC.md, or changing original tests.

## Dependencies

REM-002/003/005/007 verified; original REM-008 timing evidence; M5 16 GB;
ADR-0020 recorded before native instrumentation. Rust MSRV remains 1.95.

## Files Modified / Allowed

- `tools/bench-memory/**` (new binary, private platform module, tests, README).
- `Cargo.toml`: add this tools workspace member only; `Cargo.lock`: local tool
  package dependency edges only, reusing already locked library versions.
- Eight existing examples only: `sim-world/examples/{grid,worldgen,site,
  pathfinding}_bench.rs`, `sim-core/examples/person_spawn_bench.rs`,
  `sim-ai/examples/{needs,utility_ai,utility_score}_bench.rs`.
- `docs/adr/ADR-0020-benchmark-memory-measurement.md`, this task;
  `docs/ARCHITECTURE.md` (tool boundary note), `docs/PERFORMANCE.md`;
  the remediation plan, CHRON-019..026 completion reports,
  `docs/reports/REM-008A_MEMORY.md` and `docs/reports/data/rem-008a-*`.
- `.github/workflows/ci.yml`: add the native CLI/probe tests to the existing
  macOS job. The existing Rust job is Linux-only, so its all-target tests cannot
  exercise Mach counters. This is test coverage for the approved tool, not a
  new required job, permission, performance threshold or remote-setting change.
- No CI relaxation: retain every existing step and required-check name.

## API Contract

No public simulation API change. Each example adds only
`pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64` for the
measurement binary's source modules. Call `observe` exactly twice: immediately
before operation and after correctness checks while its result is still live.
Return a deterministic checksum. Unsupported fixture selectors fail explicitly.
No observer in original timing paths. Memory tool owns native reads, child
isolation, JSON and aggregation; adapters do not depend on it.

## Tests

- Unit tests of exact/ambiguous interval proof, overflow/invalid/regressing data.
- Fresh-child integration probes: 64 MiB touched then released must leave a
  recorded high-water peak substantially above its ending RSS; contaminated
  prepared interval must be marked unavailable, never numeric success.
- Invalid case/count, subprocess failure, per-case checksum stability and
  process isolation. macOS-specific probes are explicitly platform-specific;
  unsupported platforms return an error, not a successful zero measurement.
- Parent independently reviews all diffs and runs fmt, warnings-denied Clippy,
  workspace tests, MSRV, doctests/docs and existing Rust smoke gates.

## Benchmark

M5, release, sequential: 22 cases × three independent cold memory processes;
original eight timing benchmarks × ten post-warmup samples. Native reads and
memory subprocesses are not inserted into timed loops. Record raw per-sample
current/peak counters and checksums plus min/median/max incremental peaks.

## Definition of Done

- Accurate cold per-case peak increments exist for all 22 cases and have an
  explicit interval proof; any unavailable prepared-only measurement is
  explicitly labeled and not substituted into the cold series.
- Transient/released memory regression passes; prior peaks cannot masquerade
  as operation-only measurements. No hidden retry/discard of bad samples.
- All original assertions/timing paths and production source remain intact.
- Native exception is confined to the measurement tool and documented; core
  unsafe-forbid and dependency boundaries remain unchanged.
- Evidence/report updates and independent quality checks complete; limitations
  and sensitive tooling changes disclosed. No next task or phase implied.

## Completion

[REM-008A_MEMORY.md](../reports/REM-008A_MEMORY.md) records 22 cases × three
fresh processes (all cold and operation peaks proved), transient/contamination
regressions, 203 passing workspace tests, 2 doctests, lint/MSRV/smoke gates,
unchanged timing-path reruns and sensitive native-tool/CI boundaries.
No product behavior, performance budget or remote setting changed.
