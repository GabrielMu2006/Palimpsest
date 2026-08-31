# CHRON-023 — Static Activity Sites

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context

Record the existing static site implementation and missing M5 evidence. REM-008 changes this report only.

## Scope

Walkable, in-bounds Meal/Rest/Work value records, deterministic nearest-site query and saturating WorkCounter observation.

## Out of Scope

No economy, inventory, employment, ownership, production/consumption, site simulation, executor or optimization.

## Dependencies

CHRON-019/020 grid and generation, ADR-0013/0014/0017; serde/serde_json as locked.

## Files Modified / Allowed

Only `docs/reports/CHRON-023_ACTIVITY_SITES.md` for this measurement; shared
method/link updates are in docs/PERFORMANCE.md. Existing production code,
benchmark code, tests, lockfile and golden expectations are unchanged.

## Tests / Verification

The parent ran the final `./tools/ci-rust.sh`: read-only Master Spec hash,
workspace fmt, warnings-denied Clippy, 163 unit/integration tests (zero failed
or ignored), Rust 1.95 MSRV check and all seven existing release smoke checks
passed. `cargo test --workspace --doc` passed 2 doctests;
`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`
passed. Existing Godot integration and three-script validation also passed.
Exact shared environment, commands and source identity are in
[PERFORMANCE.md](../PERFORMANCE.md).

Covered cases: Walkability and kind/counter invariants; duplicate coordinates; each default site kind present; repeat placement; nearest/distance and row-major tie behavior; checked work updates/saturation; serde round trips/rejection.

## Benchmark — M5 16 GiB, 2026-08-30

Base e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted remediation;
release build, macOS 26.6.2, rustc/cargo 1.98.0. Samples are sequential with no
concurrent worker build/test/benchmark. Median is the sorted upper-middle
sample. See the shared environment for limitations.

```sh
/usr/bin/time -l target/release/examples/site_bench 10 2
```

A generated map with seed 0x5EED_5EED_5EED_5EED supplies 20 distinct walkable sites. Each sample runs 10,000 nearest queries and 10,000 record_work calls; ten samples after two warmups. Query timing includes deterministic query-stream generation; work timing includes cloning the fixture and summing counters. Nanoseconds per op are integer-floor averages of whole batches before the reported median, not individually timed calls.

Nearest query min/median/max: 22 / 26 / 26 ns per op. Work recording: 15 / 18 / 19 ns per op. Query checksum 81748317; every work batch asserts exactly 10,000 recorded advances.

Whole-command max resident set size: 1884160 B; peak memory footprint:
1245520 B, as reported by macOS /usr/bin/time -l. These include setup and
all workloads, not an isolated per-operation incremental peak. Internal
rss_delta_bytes is a before/after retained-state ps sample (KiB ×1024), not
peak allocation. A zero delta does not mean zero memory.

Exact aggregate stdout (the harness does not emit individual samples):

```jsonl
{"sites":20,"samples":10,"query_ops_per_sample":10000,"advance_ops_per_sample":10000,"find_nearest_min_ns_per_op":22,"find_nearest_median_ns_per_op":26,"find_nearest_max_ns_per_op":26,"record_work_min_ns_per_op":15,"record_work_median_ns_per_op":18,"record_work_max_ns_per_op":19,"rss_delta_bytes":81920,"find_nearest_checksum":81748317}
```

## Definition of Done / Known Limitations / Blockers

Correctness gates and the required ten-sample timing workload passed; this
report supplies reproducible results, exact aggregate output and test coverage.
Static nearest lookup is not terrain reachability. WorkCounter is observation only, not production or successful person action execution.

Precise per-workload peak incremental RSS is **not measured** by the existing
harness. Retained deltas and whole-command peaks are supplied, not substituted
as a false pass for that stronger metric. The peak-incremental requirement
remains an explicit REM-008 measurement gap; no budget or test was relaxed.
This is not the Phase 1 kernel or 100-NPC/10-year acceptance gate.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`sites`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-world/examples/site_bench.rs`, shared outward-only measurement
tool, native regression tests and report/CI integration under
[REM-008A](../tasks/REM-008A.md) and [ADR-0020](../adr/ADR-0020-benchmark-memory-measurement.md).
Dependencies are the existing verified component implementation; no production
behavior or original timing/assertion was changed. No optimization, budget
change, CHRON-027 or Phase 1 kernel validation is included.

Authoritative min/median/max, exact interval boundaries, source hashes,
reproduction commands, final checks and limitations:
[REM-008A completion report](REM-008A_MEMORY.md). Raw
[peak samples](data/rem-008a-memory.jsonl) and
[follow-up timing output](data/rem-008a-timing.jsonl) preserve all runs.
Use this follow-up for current peak coverage; earlier sections remain staged
historical evidence. Zero extra resident pages is not zero object memory,
and cold fixture cost is not prepared-operation cost.

