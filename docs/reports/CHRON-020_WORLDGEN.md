# CHRON-020 — Deterministic World Generation

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context

Record the existing minimal deterministic world generator and the missing M5 evidence. REM-008 changes this report only.

## Scope

Version-1 generator, u64 WorldSeed, Ground/Water/Rock walkability and one 128×128 local map; validated spawn configuration and provenance.

## Out of Scope

No ecology, resources, climate, species, multiple regions, terrain evolution, generation changes or optimization.

## Dependencies

CHRON-019's coordinate/grid contract, ADR-0013/0017; serde 1.0.229 and serde_json 1.0.151.

## Files Modified / Allowed

Only `docs/reports/CHRON-020_WORLDGEN.md` for this measurement; shared
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

Covered cases: Same-seed cell/byte equality; distinct and zero seeds; golden hashes; default connected spawn and impassable feature; custom spawn size; exact shape; walkability; provenance; invalid version/spawn config; serde round trips.

## Benchmark — M5 16 GiB, 2026-08-30

Base e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted remediation;
release build, macOS 26.6.2, rustc/cargo 1.98.0. Samples are sequential with no
concurrent worker build/test/benchmark. Median is the sorted upper-middle
sample. See the shared environment for limitations.

```sh
/usr/bin/time -l target/release/examples/worldgen_bench 10 2
```

Seeds 0, 1 and 42, default generator version 1 with minimum 64 walkable spawn cells, ten samples after two warmups per seed. Timed work is WorldMap::generate; serialization occurs outside the timer. map_json_bytes is the local terrain grid JSON only, not an entire save or WorldMap provenance envelope. Deterministic cell comparisons remain enabled.

Seed 0: median 594,375 ns, 138,863 JSON B. Seed 1: 339,791 ns, 140,838 B. Seed 42: 291,083 ns, 137,170 B. Exact min/max, RSS deltas and full-width FNV hashes are retained below.

Whole-command max resident set size: 1998848 B; peak memory footprint:
1376592 B, as reported by macOS /usr/bin/time -l. These include setup and
all workloads, not an isolated per-operation incremental peak. Internal
rss_delta_bytes is a before/after retained-state ps sample (KiB ×1024), not
peak allocation. A zero delta does not mean zero memory.

Exact aggregate stdout (the harness does not emit individual samples):

```jsonl
{"seed":0,"samples":10,"gen_min_ns":578833,"gen_median_ns":594375,"gen_max_ns":691458,"map_json_bytes":138863,"rss_delta_bytes":49152,"fnv1a":10103231413028631179}
{"seed":1,"samples":10,"gen_min_ns":339417,"gen_median_ns":339791,"gen_max_ns":407917,"map_json_bytes":140838,"rss_delta_bytes":0,"fnv1a":9466269938330766210}
{"seed":42,"samples":10,"gen_min_ns":290875,"gen_median_ns":291083,"gen_max_ns":337375,"map_json_bytes":137170,"rss_delta_bytes":0,"fnv1a":8056959030977719378}
```

## Definition of Done / Known Limitations / Blockers

Correctness gates and the required ten-sample timing workload passed; this
report supplies reproducible results, exact aggregate output and test coverage.
One deterministic preview map is not an ecological or geographical world. Warm-up and OS scheduling variation are visible across these very short operations; do not infer a change in generator behavior from timing variation.

Precise per-workload peak incremental RSS is **not measured** by the existing
harness. Retained deltas and whole-command peaks are supplied, not substituted
as a false pass for that stronger metric. The peak-incremental requirement
remains an explicit REM-008 measurement gap; no budget or test was relaxed.
This is not the Phase 1 kernel or 100-NPC/10-year acceptance gate.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`worldgen-0, worldgen-1, worldgen-42`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-world/examples/worldgen_bench.rs`, shared outward-only measurement
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

