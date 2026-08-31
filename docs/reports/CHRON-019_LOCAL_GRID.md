# CHRON-019 — Local Grid Baseline

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context

Record the existing generic LocalCoord / LocalGrid<T> / WorldGrid<T> implementation and its missing M5 evidence. REM-008 adds this report only; no grid code changed.

## Scope

One 128×128, 16,384-cell container; validated coordinates, fallible access, row-major iteration and serde shape.

## Out of Scope

No terrain/generation/pathfinding changes, ECS, extra regions, simulation behavior or optimization.

## Dependencies

CHRON-018, ADR-0017 and the existing LocalCoord/grid contract; serde 1.0.229.

## Files Modified / Allowed

Only `docs/reports/CHRON-019_LOCAL_GRID.md` for this measurement; shared
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

Covered cases: Corners and out-of-bounds coordinates; total index/coordinate inverse; row-major ordering/hash; exact cell counts; fallible access/swap; deterministic iteration; serde round trips and rejection of invalid coordinates/lengths.

## Benchmark — M5 16 GiB, 2026-08-30

Base e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted remediation;
release build, macOS 26.6.2, rustc/cargo 1.98.0. Samples are sequential with no
concurrent worker build/test/benchmark. Median is the sorted upper-middle
sample. See the shared environment for limitations.

```sh
/usr/bin/time -l target/release/examples/grid_bench 10 2
```

Ten samples after two warmups. The input Vec<u64> is prepared outside the construction timer; construction measures validation/ownership transfer, not allocation/filling. Full scan uses 16,384 sequential get calls and black_box, with checksum 2041721 asserted. The payload alone is analytically 131,072 B, not the measured process-RSS delta.

Construction min/median/max: 0 / 41 / 83 ns. Full scan: 11,209 / 11,291 / 11,375 ns. Construction is at timer-resolution scale; 0 ns does not mean free work.

Whole-command max resident set size: 1851392 B; peak memory footprint:
1212752 B, as reported by macOS /usr/bin/time -l. These include setup and
all workloads, not an isolated per-operation incremental peak. Internal
rss_delta_bytes is a before/after retained-state ps sample (KiB ×1024), not
peak allocation. A zero delta does not mean zero memory.

Exact aggregate stdout (the harness does not emit individual samples):

```jsonl
{"cells":16384,"samples":10,"warmups":2,"build_min_ns":0,"build_median_ns":41,"build_max_ns":83,"scan_min_ns":11209,"scan_median_ns":11291,"scan_max_ns":11375,"rss_before_bytes":1736704,"rss_after_bytes":1802240,"rss_delta_bytes":65536,"checksum":2041721}
```

## Definition of Done / Known Limitations / Blockers

Correctness gates and the required ten-sample timing workload passed; this
report supplies reproducible results, exact aggregate output and test coverage.
The container is a single fixed local grid. The narrow constructor timing cannot predict world-generation cost. No Phase 1 kernel throughput claim.

Precise per-workload peak incremental RSS is **not measured** by the existing
harness. Retained deltas and whole-command peaks are supplied, not substituted
as a false pass for that stronger metric. The peak-incremental requirement
remains an explicit REM-008 measurement gap; no budget or test was relaxed.
This is not the Phase 1 kernel or 100-NPC/10-year acceptance gate.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`grid`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-world/examples/grid_bench.rs`, shared outward-only measurement
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

