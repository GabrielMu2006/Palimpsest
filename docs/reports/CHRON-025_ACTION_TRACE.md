# CHRON-025 — Action Candidates and Decision Trace

> Current peak-memory evidence: [REM-008A](REM-008A_MEMORY.md) closes the earlier
> measurement gap. The original REM-003/005/007/008 sections below are historical
> stage records; the final follow-up section describes the new tooling scope.

## Context / Scope

This records the existing CHRON-025 data-construction implementation and the
authorized REM-007 correction under accepted ADR-0019. The provider enumerates
only Move, Eat, Sleep, Work and Idle from one immutable local context; every
candidate exposes the same five factor inputs. This is not an action executor.

REM-007 adds validated individual construction and diagnostic identity checks.
The parent retained this task locally because correspondence across duplicated
Selection fields and partial traces needs coordinated review. It was not
delegated to another implementation worker.

## Out of Scope

No movement/execution, Needs changes, new actions/factors, AI depth, persistence,
Event Store, bridge, dependencies, or CHRON-027 work. Imported diagnostics are
not simulation truth, commands, or proof of historical score correctness.

## Dependencies

CHRON-019..024; ADR-0013/0014/0017 and accepted ADR-0019. REM-005's accepted
default-weight change preceded this implementation and remains unchanged.

## Files Modified / Allowed

REM-007 changes action.rs, trace.rs, utility.rs and public re-exports in
`crates/sim-ai/src/lib.rs`, including their inline tests; this report and
CHRON-026's report. Existing example call sites already used `.expect` and
required no edits. No manifest, lockfile, world/core/bridge implementation,
Task specification or ADR was edited by REM-007.

## API Contract / Changes

- `ActionCandidate::new` returns `Result<_, CandidateError>`: Idle has no
  target; every other kind has one. Serde uses the same validation and retains
  the existing field/enum names. Individual keys can be any u64.
- A shared three-pass collection validator reports the first duplicate order,
  then first out-of-range order, then first repeated kind/target. Complete
  selection keys must be exactly the set `0..len`; input vector permutation
  remains legal. It never scans to an attacker-supplied maximum key.
- `DecisionTrace::new` returns `Result<_, TraceValidationError>`. Unselected
  fragments may be empty or non-contiguous but cannot duplicate identities.
  `trace_for` still returns a single fragment directly, including order 6.
- Selected trace decoding requires a nonempty complete set, an existing
  selected key and a tie reason. Unselected fragments cannot have a tie reason.
- Selection decoding checks complete all-scores identity, chosen key/candidate
  and total, and equality of per-key candidate traces and repeated total/
  perturbation fields. Copies are checked by key, not vector position.
- `score_candidates` retains its Vec-returning diagnostic-subset API.
  Contextual site feasibility is still the provider/executor's responsibility.
  Perturbation changes and the performance comparison are in CHRON-026's report.

## Tests / Verification

The parent first added `malformed_candidate_wire_is_rejected` and the matching
perturbation regression, then ran `cargo test -p palimpsest-sim-ai malformed_`.
Both failed under the old derived deserializers (exit 101, 2 failed). Both pass
under the corrected implementation; no test was disabled or removed.

Additional coverage: all 5 kinds × 2 target forms × keys 0/1/u64::MAX through
native and serde paths; invalid coordinates; complete [0,1]/[1,0] permutations;
duplicate [0,0], gaps [0,2]/[1]/[0,MAX], duplicate kind/target and deterministic
error precedence; partial empty/6/MAX keys; duplicate fragment identities;
selected trace key/tie errors; separately corrupted Selection copies; valid
provider/Selection round trips and zero/bounded seeded equivalence. Existing
candidate feasibility, ordering, boundedness, five-factor completeness,
integer, deterministic, serde and Utility saturation tests remain.

Commands actually run by the parent after implementation:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
```

All passed: 163 workspace unit/integration tests (sim-ai 60), zero failed or
ignored, and 2 core doctests. Full final CI/integration and benchmark evidence
are recorded in the REM-008 section when measured.

## Benchmark

Candidate enumeration and full trace construction use the existing
`utility_ai_bench` at 100/1000 persons, six sites and ten post-warm-up samples.
Measured output and memory-method qualifications are appended below.

## Definition of Done / Limitations

The candidate/trace contract and specified validation regressions pass without
changing provider ordering or introducing execution. Structural correspondence
does not authenticate a trace, recompute factors/scores against historical
world state, or bound arbitrary imported JSON size. Diagnostic fragments stay
read-only and are not durable save records. Final benchmark-method coverage is
reported separately; this report does not declare the Phase 1 kernel complete.

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

The fixed map uses seed 0 and six default sites. Persons are plain location/
Needs pairs, strided across all walkable cells; not all reach every site.
There are 861 candidates at 100 persons and 9,060 at 1,000. Enumeration
medians: 40.004 / 364.870 ms (21,523.072 / 24,830.747 candidates/s).
Full-trace medians: 59.643 / 554.274 ms (14,435.944 / 16,345.696 traces/s).

The trace timing includes re-enumeration plus trace_for for every candidate,
including reachability queries and correctness assertions. It is not pure
trace allocation cost; do not add it to enumeration time as if independent.
The retained-memory stage stores all single-candidate traces.

```sh
/usr/bin/time -l target/release/examples/utility_ai_bench 10 2
```

Exact aggregate stdout (not individual samples):

```jsonl
{"persons":100,"sites":6,"samples":10,"candidates_total":861,"enumeration_min_ns":39652625,"enumeration_median_ns":40003583,"enumeration_max_ns":41157625,"candidates_per_second":21523.072,"traces_total":861,"trace_min_ns":59007458,"trace_median_ns":59642792,"trace_max_ns":61038500,"full_traces_per_second":14435.944,"rss_delta_bytes":360448,"enumeration_checksum":6214235,"trace_checksum":889859}
{"persons":1000,"sites":6,"samples":10,"candidates_total":9060,"enumeration_min_ns":363400792,"enumeration_median_ns":364870208,"enumeration_max_ns":365328208,"candidates_per_second":24830.747,"traces_total":9060,"trace_min_ns":551219875,"trace_median_ns":554274333,"trace_max_ns":555968083,"full_traces_per_second":16345.696,"rss_delta_bytes":2457600,"enumeration_checksum":65932654,"trace_checksum":9748469}
```

macOS /usr/bin/time -l reported max resident set size 5406720 B
and peak memory footprint 4735312 B for the entire command, including
setup and all scales. Per-row rss_delta_bytes is a retained warm-process
before/after ps sample, not peak allocation. Zero does not imply zero memory.

Precise per-workload peak incremental RSS is not measured by this harness;
that stronger requirement remains a documented REM-008 gap. No value is
silently substituted or waived. The 100-NPC/10-year gate is not run here.

## REM-008A Peak Measurement Follow-up — 2026-08-30

The owner included the tooling extension after the historical REM-008 report.
The precise peak-incremental RSS measurement gap is now closed for the cases
`candidates-100, candidates-1000`: three fresh-process samples each, with both cold fixture-plus-
operation and prepared-operation peaks proved from macOS kernel counters.
No earlier ps delta or whole-command peak was relabeled.

Scope: additive memory adapter in `crates/sim-ai/examples/utility_ai_bench.rs`, shared outward-only measurement
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
