# CHRON-024 — Local Pathfinding

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context

Record the existing deterministic bounded local pathfinder and missing M5 evidence. REM-008 changes this report only.

## Scope

Four-neighbour A*-style search within one 128×128 grid, explicit node/path limits and deterministic tie order.

## Out of Scope

No cross-region routing, dynamic avoidance, group motion, path smoothing, executor, cancellation API, algorithm changes or optimization.

## Dependencies

CHRON-019/020, ADR-0013/0017; fixed seed-42 version-1 default world, golden FNV 8056959030977719378.

## Files Modified / Allowed

Only `docs/reports/CHRON-024_PATHFINDING.md` for this measurement; shared
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

Covered cases: Valid optimal paths and detours; repeat/tie determinism; start==goal; blocked/out-of-bounds endpoints and fully impassable grids; unreachable goals; zero/limited path caps; exact node budgets; generated-map paths; no panic on invalid endpoints.

## Benchmark — M5 16 GiB, 2026-08-30

Base e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted remediation;
release build, macOS 26.6.2, rustc/cargo 1.98.0. Samples are sequential with no
concurrent worker build/test/benchmark. Median is the sorted upper-middle
sample. See the shared environment for limitations.

```sh
/usr/bin/time -l target/release/examples/pathfinding_bench
```

Seven fixed queries selected deterministically from seed-42 map components. Ten samples after two warmups per query (hardcoded in the unchanged harness). Found paths assert endpoints, adjacency, walkability, cost and cap; repeated results must equal the reference. Expansion counts are obtained outside the timer by node-budget bisection except node-limit errors, which return the count directly.

Median query times: trivial 83 ns; short 4,959 ns; medium 8,083 ns; long 29,792 ns; unreachable 909,792 ns; node budget 13,333 ns; path budget 270,000 ns. Maximum expansions 10,618; peak returned path length 251 cells. An actual cross-component unreachable pair was present.

Whole-command max resident set size: 2555904 B; peak memory footprint:
1900880 B, as reported by macOS /usr/bin/time -l. These include setup and
all workloads, not an isolated per-operation incremental peak. Internal
rss_delta_bytes is a before/after retained-state ps sample (KiB ×1024), not
peak allocation. A zero delta does not mean zero memory.

Exact aggregate stdout (the harness does not emit individual samples):

```jsonl
{"query":"trivial","seed":42,"start":[0,0],"goal":[0,0],"outcome":"found","max_nodes":16384,"max_path_len":16384,"samples":10,"min_ns":42,"median_ns":83,"max_ns":84,"nodes_expanded":0,"path_len":1,"cost":0}
{"query":"short","seed":42,"start":[0,0],"goal":[8,0],"outcome":"found","max_nodes":16384,"max_path_len":16384,"samples":10,"min_ns":4791,"median_ns":4959,"max_ns":11292,"nodes_expanded":8,"path_len":9,"cost":8}
{"query":"medium","seed":42,"start":[0,0],"goal":[34,0],"outcome":"found","max_nodes":16384,"max_path_len":16384,"samples":10,"min_ns":7958,"median_ns":8083,"max_ns":8209,"nodes_expanded":66,"path_len":41,"cost":40}
{"query":"long","seed":42,"start":[0,0],"goal":[127,123],"outcome":"found","max_nodes":16384,"max_path_len":16384,"samples":10,"min_ns":29666,"median_ns":29792,"max_ns":30375,"nodes_expanded":290,"path_len":251,"cost":250}
{"query":"unreachable","seed":42,"start":[0,0],"goal":[87,62],"outcome":"unreachable","max_nodes":16384,"max_path_len":16384,"samples":10,"min_ns":746333,"median_ns":909792,"max_ns":1316500,"nodes_expanded":10618,"path_len":null,"cost":null}
{"query":"node_budget","seed":42,"start":[0,0],"goal":[127,123],"outcome":"limit_exceeded","max_nodes":145,"max_path_len":16384,"samples":10,"min_ns":13208,"median_ns":13333,"max_ns":13709,"nodes_expanded":145,"path_len":null,"cost":null}
{"query":"path_budget","seed":42,"start":[0,0],"goal":[127,123],"outcome":"unreachable","max_nodes":16384,"max_path_len":125,"samples":10,"min_ns":265208,"median_ns":270000,"max_ns":302833,"nodes_expanded":2899,"path_len":null,"cost":null}
{"summary":"bench_pathfinding","seed":42,"generator_version":1,"queries":7,"unreachable_pair":true,"max_nodes_expanded":10618,"peak_path_len":251,"rss_delta_bytes":344064,"map_fnv1a":8056959030977719378}
```

## Definition of Done / Known Limitations / Blockers

Correctness gates and the required ten-sample timing workload passed; this
report supplies reproducible results, exact aggregate output and test coverage.
The node-budget query returns LimitExceeded at 145 expansions. The length-limited query returns Unreachable under max_path_len=125, not a partial path and not proof of global disconnection; its unconstrained path has 251 cells. No separate cancellation facility exists or was tested. Kernel per-tick path cost remains future validation.

Precise per-workload peak incremental RSS is **not measured** by the existing
harness. Retained deltas and whole-command peaks are supplied, not substituted
as a false pass for that stronger metric. The peak-incremental requirement
remains an explicit REM-008 measurement gap; no budget or test was relaxed.
This is not the Phase 1 kernel or 100-NPC/10-year acceptance gate.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`path-trivial, path-short, path-medium, path-long, path-unreachable, path-node_budget, path-path_budget`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-world/examples/pathfinding_bench.rs`, shared outward-only measurement
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

