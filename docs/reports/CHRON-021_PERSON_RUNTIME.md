# CHRON-021 Person Runtime — REM-003 Completion Report

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

Status: REM-003 independently verified by the parent Codex agent. The product
owner approved continuation of the identified remediation plan on 2026-08-30;
the full REM-008 evidence pass is recorded separately below when run.

## Context

REM-003 restores the accepted ADR-0011 contract by closing the accidental
public exposure of the provisional `bevy_ecs::Entity` runtime handle from
`PersonRuntime`.

## Scope

- Make `PersonRuntime::runtime_handle` a private, `#[cfg(test)]` helper.
- Retain internal handle uniqueness coverage in the `person` test module.
- Add documentation tests proving that the stable `EntityId` query remains
  available while external runtime-handle access fails to compile.
- Correct documentation that previously described `#[doc(hidden)]` as an
  encapsulation mechanism.

## Out of Scope

No identity redesign, CurrentAction, persistence, ECS replacement, Utility AI,
CHRON-027, or changes outside the allowed source/report files.

## Dependencies

ADR-0011, CHRON-021, and the Phase 1 remediation plan. The source edition is
workspace edition 2024.

## Files Modified / Allowed

- `crates/sim-core/src/person.rs`
- This report

`crates/sim-core/src/lib.rs` was not changed because no handle re-export was
needed.

## Change Summary

The runtime ECS handle remains available only to same-module tests under
`cfg(test)`. It is absent from the normal library API. `EntityId`, `get`, and
`location` behavior is unchanged; the ECS mapping remains internal and
non-persistent.

## Tests / Verification

Commands run from the repository root:

- `cargo test -p palimpsest-sim-core --all-targets --all-features` — passed;
  12 unit tests passed and the benchmark target built with 0 tests.
- `cargo test -p palimpsest-sim-core --doc` — passed; one normal doctest and
  one `compile_fail` doctest passed.
- `cargo clippy -p palimpsest-sim-core --all-targets --all-features -- -D warnings`
  — passed.

Covered cases: stable IDs and distinct internal handles for 100 spawns,
spawn/location, stable-identity location updates, unknown-ID no-op behavior,
unrepresentable out-of-bounds locations, allocator exhaustion, deterministic
visible state, serializable stable view only, and Needs attachment/update.

## Benchmark

Command:

`cargo run --release -p palimpsest-sim-core --example person_spawn_bench -- 10 2`

The existing benchmark performs 2 warmups and 10 measured samples, then emits
min/median/max (it does not print each raw sample). Post-change output:

| Persons | Median | Spawn/s | RSS delta | Per-person RSS |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 50,167 ns | 1,993,342.237 | 81,920 B | 819 B |
| 1,000 | 149,791 ns | 6,675,968.516 | 16,384 B | 16 B |

Observed min/max were 33,875/149,459 ns (100) and 146,375/153,583 ns
(1,000). Supplied pre-change baseline was 44,042 ns / 2,270,559.920
spawns/s / 114,688 B for 100 and 154,333 ns / 6,479,495.636 spawns/s / 0 B
for 1,000. RSS is best-effort warm-process `ps` delta and is not a hard gate;
the benchmark output does not preserve raw per-sample values.

## Definition of Done

- External callers cannot call `runtime_handle` — verified by passing
  `compile_fail` doctest.
- Internal uniqueness tests still pass.
- Public unknown-identity query returns `None` — verified by normal doctest and
  existing unit tests.
- Existing behavior and tests are preserved; no CHRON-027 work started.

## Known Limitations

Needs and CurrentAction remain intentionally scoped to their existing tasks;
Phase 2 person depth is absent; the runtime map is non-persistent; and
`bevy_ecs` remains provisional under ADR-0011. Raw benchmark samples are not
available from the existing benchmark's output format.

## Blockers

No remaining REM-003 code blocker or repeat dispatch-approval requirement.
This report does not mark the entire remediation or Phase 1 complete.

## Parent Independent Review — 2026-08-30

Dispatch used the user-requested `codex-luna-dispatch` skill. Requested model:
`gpt-5.6-luna`, medium reasoning; agent `/root/rem003_runtime_handle`. The
dispatch response exposed the agent identity but not the actual backend model,
so backend routing is not independently verified.

The parent inspected the complete diff, including this untracked report. Only
`person.rs` and this report were changed by the worker. The existing 12 core
tests were preserved; two doctests were added. Positive imports and stable-ID
lookup compile successfully; the negative doctest's forbidden call would have
compiled with the old public accessor. No production behavior, dependency,
Task approval, or accepted ADR was changed. Existing unrelated user edits
were preserved.

Commands independently run by the parent, all successful:

- `cargo fmt --all -- --check`
- `cargo test -p palimpsest-sim-core --all-targets --all-features` — 12 passed
- `cargo test -p palimpsest-sim-core --doc` — 2 passed, including compile-fail
- `cargo clippy -p palimpsest-sim-core --all-targets --all-features -- -D warnings`
- `./tools/ci-rust.sh` — read-only Master Spec hash gate; workspace fmt,
  Clippy, 151 unit/integration tests (0 failed/ignored), Rust 1.95 MSRV check,
  and the seven existing headless/scheduler/ECS/event/mode/storage/snapshot
  smoke commands all passed
- `cargo test --workspace --doc` — 2 passed
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`
- `git diff --check`

Generated `PersonRuntime` rustdoc has no `id="method.runtime_handle"` method
entry. The normal API excludes that method; its intentional negative example
still mentions its name. The Master Spec SHA-256 remains
`a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`.

### Parent Benchmark Recheck

Same machine and command before/after: Apple M5, 16 GiB unified memory,
macOS 26.6.2; rustc 1.98.0 (88d9e12ae 2026-08-18), cargo 1.98.0
(797e8a9bc 2026-08-05). Baseline commit `e5b0aeb`; result is the uncommitted
REM-003 patch on that commit. Release profile, 2 warmups, 10 measured samples
per size; reported median is the sorted upper-middle sample, as implemented
by the unchanged benchmark.

`cargo run --release -p palimpsest-sim-core --example person_spawn_bench -- 10 2`

| Persons | Parent before min/median/max ns | Parent after min/median/max ns | After spawn/s | After RSS delta |
|---:|---:|---:|---:|---:|
| 100 | 35,083 / 44,042 / 49,250 | 19,916 / 24,375 / 28,959 | 4,102,564.103 | 114,688 B |
| 1,000 | 146,250 / 154,333 / 162,334 | 73,584 / 94,459 / 105,375 | 10,586,603.712 | 16,384 B |

Exact post-change stdout (aggregate raw output; not individual samples):

```jsonl
{"persons":100,"samples":10,"spawn_min_ns":19916,"spawn_median_ns":24375,"spawn_max_ns":28959,"spawns_per_second":4102564.103,"rss_delta_bytes":114688,"per_person_bytes":1146,"checksum":10000}
{"persons":1000,"samples":10,"spawn_min_ns":73584,"spawn_median_ns":94459,"spawn_max_ns":105375,"spawns_per_second":10586603.712,"rss_delta_bytes":16384,"per_person_bytes":16,"checksum":566168}
```

Checksums match the baseline. Do not attribute the faster parent sample to a
performance improvement: production spawn code is unchanged, worker and parent
microbenchmark timings vary substantially, and CPU/thermal/background-load
conditions were not controlled. RSS measures retained warm-process deltas,
not total person allocation; a zero baseline delta does not mean zero memory.
The smoke run found no gross regression, but is not the full REM-008 benchmark
acceptance. No thresholds were relaxed. Godot rendering/FPS was not rerun for
this internal Rust API correction; the existing bridge compiled in workspace
gates, and final Godot integration verification remains REM-008.

## REM-008 Final Measurement — 2026-08-30

Measured after the final code fix on Apple M5 (10 cores), 16 GiB unified
memory, macOS 26.6.2; rustc/cargo 1.98.0, release profile. Base commit
e5b0aeb676372a123dd8c27190e94b6a606d498c plus uncommitted remediation.
Ten samples after two warmups per case; sorted upper-middle median. No
worker compilation or other benchmark ran concurrently. Exact build commands,
source hashes and common limitations are in [PERFORMANCE.md](../PERFORMANCE.md).

Final correctness evidence: ./tools/ci-rust.sh passed its hash, fmt, Clippy,
163 unit/integration tests, Rust 1.95 check and seven release smoke checks.
Two doctests and warnings-denied rustdoc passed. ./tools/ci-godot.sh loaded
Godot-rust successfully under Godot 4.7.2; gda validated all three scripts in
the correct project with no diagnostics. This is existing bridge integration,
not a rendered Micro World/FPS test or a Phase 1 kernel acceptance.

The measured runtime contains the current Person marker, stable-ID mapping,
Location and existing CHRON-022 Needs attachment. It is not the original
marker/Location-only historical snapshot. Timer includes runtime/allocator
creation and spawning; checksum observation occurs outside the timer.

100 persons: median 41,125 ns (2,431,610.942 spawns/s), retained delta 49,152 B,
floor per-person delta 491 B. 1,000: 148,750 ns (6,722,689.076/s), sampled delta
0 B. These figures do not isolate the mapping's allocation; mapping-only
memory is N/A, not required to claim the measured whole-runtime baseline.

```sh
/usr/bin/time -l target/release/examples/person_spawn_bench 10 2
```

Exact aggregate stdout (not individual samples):

```jsonl
{"persons":100,"samples":10,"spawn_min_ns":29333,"spawn_median_ns":41125,"spawn_max_ns":57667,"spawns_per_second":2431610.942,"rss_delta_bytes":49152,"per_person_bytes":491,"checksum":10000}
{"persons":1000,"samples":10,"spawn_min_ns":145125,"spawn_median_ns":148750,"spawn_max_ns":160333,"spawns_per_second":6722689.076,"rss_delta_bytes":0,"per_person_bytes":0,"checksum":566168}
```

macOS /usr/bin/time -l reported max resident set size 3129344 B
and peak memory footprint 2310480 B for the entire command, including
setup and all scales. Per-row rss_delta_bytes is a retained warm-process
before/after ps sample, not peak allocation. Zero does not imply zero memory.

The unchanged production spawn path and variable earlier parent/worker results
show why the timing changes are not an optimization claim. Stable checksums
remain 10000/566168. REM-003 has no code blocker; this report does not establish
10K scaling, persistence of runtime handles, or final Phase 1 acceptance.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`person-100, person-1000`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-core/examples/person_spawn_bench.rs`, shared outward-only measurement
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
