# CHRON-022 — Fixed-Point Needs

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context

Record the existing Hunger/Fatigue model and its missing M5 baseline. REM-008 changes this report only; REM-005 changed scoring, not Needs rates.

## Scope

Bounded integer Needs advanced only by explicit SimDuration; clamped eat/rest, pressure projection, deterministic serde; existing stable-ID PersonRuntime attachment.

## Out of Scope

No personality, diet/resources, death/starvation penalties, action execution, new rates, 10-year collective simulation or optimization.

## Dependencies

CHRON-005 time/SimDuration (ADR-0003), CHRON-004 stable identity,
CHRON-021 person runtime, ADR-0013/0014/0017. sim-ai depends on
sim-time/sim-world/serde, not ECS/core.

## Files Modified / Allowed

Only `docs/reports/CHRON-022_NEEDS.md` for this measurement; shared
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

Covered cases: Satisfied defaults; zero/monotonic advance; saturation/overflow; integer associativity; eat/rest clamp without underflow; pressure/critical bounds; repeat determinism; native/serde invalid boundaries; stable-ID attachment/update.

## Benchmark — M5 16 GiB, 2026-08-30

Base e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted remediation;
release build, macOS 26.6.2, rustc/cargo 1.98.0. Samples are sequential with no
concurrent worker build/test/benchmark. Median is the sorted upper-middle
sample. See the shared environment for limitations.

```sh
/usr/bin/time -l target/release/examples/needs_bench 10 2
```

100 and 1,000 plain Needs records; hourly advances through 365 days: 8,760 advances per person-year, 876,000 / 8,760,000 calls per sample. Ten samples after two warmups. Timer includes vector initialization and hourly loops; black_box retains each update, and the harness asserts both drives saturate without eating/resting. This is low-level quantity arithmetic, not autonomous NPC activity.

100 persons: median 1,016,666 ns, 861,639,909.272 advances/s. 1,000 persons: 5,367,584 ns, 1,632,019,172.872 advances/s. Updates per person-year: exactly 8,760 at this benchmark cadence.

Whole-command max resident set size: 1769472 B; peak memory footprint:
1114424 B, as reported by macOS /usr/bin/time -l. These include setup and
all workloads, not an isolated per-operation incremental peak. Internal
rss_delta_bytes is a before/after retained-state ps sample (KiB ×1024), not
peak allocation. A zero delta does not mean zero memory.

Exact aggregate stdout (the harness does not emit individual samples):

```jsonl
{"persons":100,"samples":10,"advances_per_person_year":8760,"advances_total":876000,"year_min_ns":898834,"year_median_ns":1016666,"year_max_ns":1097834,"advances_per_second":861639909.272,"rss_delta_bytes":65536}
{"persons":1000,"samples":10,"advances_per_person_year":8760,"advances_total":8760000,"year_min_ns":4846167,"year_median_ns":5367584,"year_max_ns":6946792,"advances_per_second":1632019172.872,"rss_delta_bytes":0}
```

## Definition of Done / Known Limitations / Blockers

Correctness gates and the required ten-sample timing workload passed; this
report supplies reproducible results, exact aggregate output and test coverage.
The real kernel cadence is not chosen here. Saturated integer updates dominate this deliberately simple one-year workload. These rates cannot be extrapolated to complete AI/movement/event loops.

Precise per-workload peak incremental RSS is **not measured** by the existing
harness. Retained deltas and whole-command peaks are supplied, not substituted
as a false pass for that stronger metric. The peak-incremental requirement
remains an explicit REM-008 measurement gap; no budget or test was relaxed.
This is not the Phase 1 kernel or 100-NPC/10-year acceptance gate.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`needs-100, needs-1000`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-ai/examples/needs_bench.rs`, shared outward-only measurement
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
