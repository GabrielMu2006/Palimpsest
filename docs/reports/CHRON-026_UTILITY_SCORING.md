# CHRON-026 REM-005 — Utility Scoring Evidence

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

The first sections record the bounded REM-005 stage. REM-007 and final REM-008
evidence are separate additions below; stage-specific non-goals and counts
are historical, not claims that later corrections are absent.

## Scope

Apply the accepted ADR-0018 Need/Work default policy in the existing integer
Utility scorer. Eat and Sleep use `SiteAvailable = 0`; Work uses `2,300`.
Need, distance, Move availability, Idle, WorkProgress, perturbation, candidate
enumeration, stable ordering, saturation, and trace behavior are unchanged.

## Out of Scope

Action execution, movement, rate changes, new factors/actions, ADR-0019
validation or wire changes, Phase 2 systems, optimization, and dependency/CI
changes.

## Dependencies

CHRON-022 Needs, CHRON-023 site availability, CHRON-024 pathfinding,
CHRON-025 candidate/trace contracts, ADR-0013, ADR-0014, and accepted
ADR-0018. ADR-0019 remains a later task and is not implemented here.

## Files Modified / Allowed

- `crates/sim-ai/src/utility.rs`
- `docs/tasks/CHRON-026.md`
- this report

No other source, API, manifest, lockfile, or specification file was changed.

## Change Summary

The default table now scores Eat/Sleep as `10 × pressure − 5 × distance` and
reachable Work as `2,300 − 5 × distance`. The documented conservative bounds
are base `[-1,270, 10,000]` and total `[-1,370, 10,100]`. Added evidence covers
the real one-second Needs advance regression, pressure sweep `0..=1000`, the
228/229/230 crossover and stable tie, low raw needs, equal and unequal high
axes at 699/700/900/1000, reachable distance sensitivity, and no-site Idle.

## Tests

The parent independently restored only the old three default availability
weights temporarily and ran the fresh one-second regression. It failed at
the first winner assertion with `Eat` instead of `Work` (exit 101; one failed
test). The accepted table was then restored. Fresh
`Needs::default().advance(1s)` yields raw `1/2`, pressure `0/0`, and selects
Work. The separate near-boundary elapsed-time case is retained as well.

Commands run:

```text
cargo test -p palimpsest-sim-ai one_second_advance_still_selects_work
rustfmt --edition 2024 crates/sim-ai/src/utility.rs
cargo test -p palimpsest-sim-ai --all-targets --all-features
cargo clippy -p palimpsest-sim-ai --all-targets --all-features -- -D warnings
cargo test -p palimpsest-sim-ai --doc
git diff --check
```

The first command intentionally produced the documented red result. Worker
delivery passed 49/49 tests; parent completion adds the missing cases below.

Coverage retained and verified: integer-only JSON scoring; base arithmetic and
zero-weight trace factors; saturating extremes; zero perturbation; bounded,
seeded perturbation; stable-order ties and unique maxima; complete traces and
all-scores; deterministic repeated and byte-identical serialization; empty
candidate errors; serde round trips; and the five-action selection contract.
Tests do not execute actions or claim a runtime execution closed loop;
CHRON-027 remains the future execution task.

## Benchmark

Command:

```text
cargo run --release -p palimpsest-sim-ai --example utility_score_bench -- 10 2
```

Reference context: Apple Silicon M5, 16 GiB, macOS 26.6.2, rustc 1.98.0;
2 warm-up iterations and 10 samples, 10 candidates per selection, 6 sites.

| persons | ε | min ns | median ns | max ns | selections/s | RSS delta | checksum |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 100 | 0 | 27,012,416 | 27,419,958 | 27,472,791 | 3,646.979 | 557,056 | 627,687 |
| 100 | 25 | 27,114,000 | 27,252,625 | 27,581,542 | 3,669.371 | 0 | 628,808 |
| 1,000 | 0 | 246,846,333 | 247,308,834 | 248,240,208 | 4,043.527 | 4,980,736 | 6,372,907 |
| 1,000 | 25 | 247,384,917 | 248,071,709 | 249,196,959 | 4,031.092 | 0 | 6,381,399 |

RSS is warm-retained delta, not peak RSS; the benchmark does not establish
zero allocation. Checksum changes are expected from the policy change, not an
optimization claim.

## Definition of Done

Accepted thresholds are implemented without API or candidate/trace changes;
old tests remain and their policy-dependent exact values are updated; the
real Needs provider and candidate feasibility are exercised. Both hunger and
fatigue sweeps cover `0..=1000`, with scores, unique maxima, stable tie at
229, repeated equality, low raw pairs, and an explicit `290 > 2×100` margin
assertion. Required tests, clippy, docs, formatting, and diff checks pass; the
benchmark was run before these test-only additions and weights are unchanged.

## Known Limitations / Blockers

Only the five Phase 1 actions are covered. This task selects and traces an
action but does not execute it; a true execution closed loop remains a future
obligation defined by CHRON-026/ADR-0018. The low-need guarantee is mathematically
bounded by the documented epsilon margin; sampled seeds are not a universal
proof. No blocker remains for parent review.

## Parent Review / Completion of REM-005

Requested worker: `gpt-5.6-luna`, medium, `/root/rem005_utility_policy`;
backend routing was not exposed by the dispatcher. The worker owned only
utility.rs, CHRON-026's task note and this report. One bounded rework was
followed by parent takeover of remaining test gaps: the worker's low-pair
inputs confused raw quantities with pressure, and its far-site fixture did
not yet exercise pressure 900. These gaps were not accepted as passing
coverage merely because the existing tests were green.

The parent completed the 4×4 grid of pressures 0/1/199/200 (multiplying by
100 to obtain raw Needs), retained the small raw cases, checked both full
0..1000 sweeps and repeated selections, and based the 290-point margin on
actual scores of both need candidates. Real reachable Meal and Rest sites
at distance >=20 now exercise critical pressure 900; their scores and
availability are asserted, with the worst-distance bound 7,730 versus
maximum Work 2,300 proving the perturbation margin. No action is executed.

The existing tests remain; policy-dependent numeric expectations follow the
accepted table. A warnings-denied test-length failure in the parent patch
was fixed by splitting the new hunger, fatigue and margin checks into
focused tests, not suppressing the lint or dropping assertions.

Parent acceptance: sim-ai 51/51 and sim-core 12/12; full workspace 154/154
unit/integration tests, 2 doctests, workspace fmt, warnings-denied Clippy and
rustdoc, and diff checks all passed. The parent independently observed the
old-table red result described above and the restored-table green result.

### Parent pre-REM-007 Baseline

After all checks finished and before any REM-007 changes, the parent ran the
prebuilt release executable sequentially:
`/usr/bin/time -l target/release/examples/utility_score_bench 10 2`.
No source/benchmark changes were made during this measurement. Baseline
utility.rs SHA-256: `2ac8a0ee1d293bde00aabdd04faf61fd1e30a355a792fe1f0219d6ab1484e8a2`.

| Persons | ε | Min / median / max ns | Selections/s | Candidates/s | Retained RSS delta B | Checksum |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 0 | 27230167 / 27432833 / 27644167 | 3645.267 | 36452.670 | 557056 | 627687 |
| 100 | 25 | 27384166 / 27464042 / 27577875 | 3641.125 | 36411.246 | 0 | 628808 |
| 1000 | 0 | 247910625 / 248522750 / 248903667 | 4023.776 | 40237.765 | 4980736 | 6372907 |
| 1000 | 25 | 247835750 / 248607542 / 249588292 | 4022.404 | 40224.041 | 0 | 6381399 |

The whole executable reported max resident set size 8,437,760 B and peak
memory footprint 7,766,352 B. These are whole-process peaks across all four
rows and setup, not per-row incremental peaks. Timers exclude candidate-list
preparation but include factor-input reachability checks and trace creation.
All timing rows are 10 samples after 2 warmups, upper-middle median. This
baseline isolates later validation changes from the earlier weight change;
timing noise still prevents attributing small differences to validation alone.

## REM-007 — Validated Perturbation and Selection

The parent implemented accepted ADR-0019 locally after REM-005 passed. The
corresponding candidate/trace API details, file whitelist and shared regression
evidence are in [CHRON-025](CHRON-025_ACTION_TRACE.md).

`PerturbationSpec::new` now returns `Result<_, PerturbationError>` rather than
Option. Invalid epsilon is rejected by native construction, standalone range
decoding, spec decoding and nested input decoding. Negative values, 101,
integer extremes, fractional JSON and overflow are tested. Zero and Bounded(0)
retain distinct wire representations and exactly zero numerical effect. The
execution-time clamp is gone; the unchanged mixer uses validated settings.

Selection rejects invalid complete candidate sets before scoring. Its public
Result signature, weights, integer arithmetic, diagnostic `score_candidates`
signature, input-order output and stable-key tie precedence remain. Native
selectors validate identities before constructing their private output;
deserialized Selection additionally checks duplicated per-key copies.

Both malformed-wire regressions failed against the old implementation and
passed after correction. Parent workspace fmt, warnings-denied Clippy and
rustdoc, 163 unit/integration tests and 2 doctests passed. All original tests
remain with explicit Result handling; the old invalid-range assertions now
assert the typed error rather than Option::None. No dependencies or benchmark
sources changed. The final validation overhead is measured below against the
parent's post-REM-005/pre-REM-007 baseline, not the old scoring policy.

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

Seed 0, six sites; 100 and 1,000 persons prefiltered to reach all sites,
exactly 10 candidates per selection. Candidate lists/context preparation are
outside the timer; select_action, identity validation, all five factors,
reachability queries, scoring and complete trace construction are inside.
Perturbation is zero or epsilon 25 with seed 26026. Correctness assertions
and repeated winner checksums remain enabled.

100-person round: 26.539 / 26.490 ms at epsilon 0/25; 1,000: 239.206 /
239.273 ms. This is not simple multiply/add throughput. Candidate enumeration
is an additional cost; the candidate benchmark uses a different location
corpus, so its per-person times must not be naively summed as a matched test.

```sh
/usr/bin/time -l target/release/examples/utility_score_bench 10 2
```

Exact aggregate stdout (not individual samples):

```jsonl
{"persons":100,"sites":6,"perturbation_epsilon":0,"samples":10,"candidates_per_selection":10,"selection_min_ns":26418458,"selection_median_ns":26539417,"selection_max_ns":26575792,"selections_per_second":3767.980,"candidates_scored_per_second":37679.803,"rss_delta_bytes":573440,"selection_checksum":627687}
{"persons":100,"sites":6,"perturbation_epsilon":25,"samples":10,"candidates_per_selection":10,"selection_min_ns":26304709,"selection_median_ns":26490292,"selection_max_ns":26642417,"selections_per_second":3774.968,"candidates_scored_per_second":37749.678,"rss_delta_bytes":0,"selection_checksum":628808}
{"persons":1000,"sites":6,"perturbation_epsilon":0,"samples":10,"candidates_per_selection":10,"selection_min_ns":238888708,"selection_median_ns":239206125,"selection_max_ns":239979834,"selections_per_second":4180.495,"candidates_scored_per_second":41804.950,"rss_delta_bytes":5111808,"selection_checksum":6372907}
{"persons":1000,"sites":6,"perturbation_epsilon":25,"samples":10,"candidates_per_selection":10,"selection_min_ns":238573750,"selection_median_ns":239273000,"selection_max_ns":239839125,"selections_per_second":4179.327,"candidates_scored_per_second":41793.265,"rss_delta_bytes":0,"selection_checksum":6381399}
```

macOS /usr/bin/time -l reported max resident set size 8388608 B
and peak memory footprint 7684432 B for the entire command, including
setup and all scales. Per-row rss_delta_bytes is a retained warm-process
before/after ps sample, not peak allocation. Zero does not imply zero memory.

### Validation comparison with the parent pre-REM-007 baseline

| Persons | ε | Before median ns | Final median ns | Observed time delta | Winner checksum |
|---:|---:|---:|---:|---:|---|
| 100 | 0 | 27432833 | 26539417 | -3.26% | same |
| 100 | 25 | 27464042 | 26490292 | -3.55% | same |
| 1000 | 0 | 248522750 | 239206125 | -3.75% | same |
| 1000 | 25 | 248607542 | 239273000 | -3.75% | same |

All valid winner checksums match. The modest apparent improvement is not proof
that validation is free or an optimization: cache/code-layout, thermal and
background conditions were not isolated, and pathfinding dominates this
workload. No standalone validator timing or imported-JSON decoding throughput
is claimed. The comparison detects no gross regression, not a statistical
upper bound on validation overhead.

Precise per-workload peak incremental RSS is not measured; retained deltas
and whole-command peaks are different metrics, leaving that REM-008 item open.
At ~26.5 ms for 100 decisions, a future kernel must not assume it can score
everyone every rendered frame. Decision cadence/path reuse require their own
diagnosed, approved work; none was added here. This is preliminary tuning,
not proof of a stable 100-NPC/10-year autonomous world.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`utility-100-0, utility-100-25, utility-1000-0, utility-1000-25`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-ai/examples/utility_score_bench.rs`, shared outward-only measurement
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
