# REM-008A — Isolated Peak RSS Completion Report

- Date: 2026-08-30 (Asia/Shanghai).
- Status: Complete — independently verified locally; no remote CI/publication claim.
- Authority: owner “纳入吧”, [REM-008A task](../tasks/REM-008A.md),
  [ADR-0020](../adr/ADR-0020-benchmark-memory-measurement.md).
- This closes a measurement-tool gap; it does not implement CHRON-027 or accept
  Phase 1's future 100-NPC/10-year validation.

## Context, scope and changes

Earlier REM-008 timing, retained ps deltas and whole-executable time -l peaks
could not prove per-workload peak increments. This follow-up adds one
outward-only tools workspace binary and additive adapters in the eight existing
examples. The original example bodies, helpers, timing loops and assertions
were preserved (444 added lines, zero deleted lines across the eight examples).
No simulation production source was changed in this follow-up.

Each memory sample is a fresh sequential process, without simulation warmup.
Two pairs of current/kernel-high-water readings separate cold setup+operation
from prepared-operation-only cost. A fixed callback records the operation end
while the validated result is still alive. CLI JSON is constructed afterward.

Out of scope: game behavior, optimization, budgets, persistent/API contracts,
Godot runtime, remote settings, commits/pushes/merge, CHRON-027+, Phase 2.
Dependencies: already verified REM-002/003/005/007, original eight fixtures,
macOS M5 reference machine, existing libc 0.2.189 ABI. This is not a new
simulation dependency.

## Method and regression evidence

For starting current RSS B, starting lifetime peak H0 and ending lifetime
peak H1, the interval maximum is H1 only if H0 == B or H1 > H0. Otherwise an
earlier peak could mask this interval: return null with an explicit reason.
Inconsistent/regressing counters fail. Cold ambiguity or failed child exits
fail the command; there is no retry, sample selection or silent zero fallback.

RSS fields come from Mach task_info(MACH_TASK_BASIC_INFO), not malloc byte
accounting, ps polling or physical footprint. Apple's implementation reads
the current physical-memory ledger balance and lifetime maximum:
[XNU task_info implementation](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/task.c).
Native methods are private to the tool under ADR-0020; no process attachment,
forked running ECS, injected code, permissions change or workload allocator
replacement is involved.

An initial Vec/free probe correctly **failed two regression assertions**:
macOS retained its freed 64 MiB allocation in the allocator, so RSS did not
actually fall. Tests were not weakened. The diagnostic-only probe now explicitly
maps, physically touches and unmaps a fixed 64 MiB range. In the observed
release probe:

- Starting RSS: 1,589,248 B.
- Ending RSS after unmap: 1,589,248 B.
- Ending lifetime peak: 68,698,112 B.
- Reported interval increment: **67,108,864 B**; endpoint delta: **0 B**.
- A separate prior-large-peak probe reports cold increment 67,108,864 B but
  prepared-operation increment **null / ambiguous_prior_peak**, not 64 MiB or
  zero. This explicitly exercises contamination rejection.

Seven CLI/integration tests pass in release mode: planned-case inventory,
invalid inputs, transient peak, retained bytes, prior-peak rejection,
fresh-process isolation and intentional child-failure propagation. Five
numerical proof tests and fourteen adapter tests cover the portable contracts.
Tests are also compiled through their original examples; counts are not claims
of forty distinct new requirements.

## Environment and reproduction

Apple M5, 10 cores, hw.memsize 17,179,869,184 B; macOS 26.6.2 (25G83), arm64;
16,384 B pages; Xcode 26.6 (17F113). rustc 1.98.0 (88d9e12ae), cargo 1.98.0
(797e8a9bc). Environment verified at 21:22 +08:00; measurements followed in this
local session. No worker builds/tests overlapped the recorded measurements.
Ordinary desktop/OS activity, CPU frequency and thermals were not controlled.

Base commit e5b0aeb676372a123dd8c27190e94b6a606d498c plus the uncommitted
remediation and this extension. No new commit or remote CI result is claimed.

Commands actually used, from repository root:

```sh
cargo build --release -p palimpsest-bench-memory -p palimpsest-sim-world -p palimpsest-sim-ai -p palimpsest-sim-core --bins --examples --locked
cargo test -p palimpsest-bench-memory --release --test cli --locked
target/release/palimpsest-bench-memory --child probe-transient
target/release/palimpsest-bench-memory --child probe-contaminated
target/release/palimpsest-bench-memory --run grid 3
target/release/palimpsest-bench-memory --run all 3
/usr/bin/time -l target/release/examples/grid_bench 10 2
/usr/bin/time -l target/release/examples/worldgen_bench 10 2
/usr/bin/time -l target/release/examples/person_spawn_bench 10 2
/usr/bin/time -l target/release/examples/needs_bench 10 2
/usr/bin/time -l target/release/examples/site_bench 10 2
/usr/bin/time -l target/release/examples/pathfinding_bench 10 2
/usr/bin/time -l target/release/examples/utility_ai_bench 10 2
/usr/bin/time -l target/release/examples/utility_score_bench 10 2
```

The last two arguments are ignored by pathfinding_bench, whose ten samples and
two warmups remain constants. Grid and Person timing commands were repeated
once, with both runs retained, to examine substantial session-to-session
microbenchmark variation. No “best” result replaced the primary run.

## Peak results (bytes)

All **22 cases × 3 cold samples = 66** independent PIDs completed with stable
checksums. Every cold **and prepared-operation** interval has a numeric proof;
no ambiguity, invalid reading, discarded sample or failed case occurred in the
final batch. Three samples are a baseline, not a statistical confidence bound.

Each column is min / median / max; the median is the sorted middle sample.
Raw full readings, process IDs and sample indices:
[rem-008a-memory.jsonl](data/rem-008a-memory.jsonl).

| Case | Cold fixture + operation peak increment B | Prepared-operation peak increment B |
|---|---:|---:|
| grid | 180,224 / 180,224 / 180,224 | 163,840 / 163,840 / 163,840 |
| worldgen-0 | 49,152 / 49,152 / 49,152 | 49,152 / 49,152 / 49,152 |
| worldgen-1 | 49,152 / 49,152 / 49,152 | 49,152 / 49,152 / 49,152 |
| worldgen-42 | 49,152 / 49,152 / 49,152 | 49,152 / 49,152 / 49,152 |
| person-100 | 278,528 / 278,528 / 278,528 | 262,144 / 262,144 / 262,144 |
| person-1000 | 589,824 / 589,824 / 606,208 | 573,440 / 573,440 / 589,824 |
| needs-100 | 16,384 / 16,384 / 16,384 | 16,384 / 16,384 / 16,384 |
| needs-1000 | 32,768 / 32,768 / 32,768 | 32,768 / 32,768 / 32,768 |
| sites | 245,760 / 245,760 / 245,760 | 0 / 0 / 0 |
| path-trivial | 671,744 / 688,128 / 688,128 | 0 / 0 / 0 |
| path-short | 704,512 / 704,512 / 704,512 | 16,384 / 16,384 / 16,384 |
| path-medium | 688,128 / 704,512 / 720,896 | 16,384 / 16,384 / 32,768 |
| path-long | 688,128 / 688,128 / 704,512 | 16,384 / 16,384 / 16,384 |
| path-unreachable | 737,280 / 737,280 / 753,664 | 65,536 / 65,536 / 65,536 |
| path-node_budget | 688,128 / 688,128 / 704,512 | 16,384 / 16,384 / 16,384 |
| path-path_budget | 753,664 / 753,664 / 753,664 | 65,536 / 65,536 / 65,536 |
| candidates-100 | 1,081,344 / 1,114,112 / 1,130,496 | 770,048 / 802,816 / 819,200 |
| candidates-1000 | 3,571,712 / 3,588,096 / 3,751,936 | 3,227,648 / 3,244,032 / 3,407,872 |
| utility-100-0 | 1,458,176 / 1,507,328 / 1,638,400 | 606,208 / 606,208 / 606,208 |
| utility-100-25 | 1,572,864 / 1,589,248 / 1,622,016 | 606,208 / 606,208 / 606,208 |
| utility-1000-0 | 6,602,752 / 6,619,136 / 6,799,360 | 5,554,176 / 5,570,560 / 5,570,560 |
| utility-1000-25 | 6,651,904 / 6,733,824 / 6,782,976 | 5,537,792 / 5,554,176 / 5,570,560 |

Fixture and lifetime definitions are detailed in the
[tool README](../../tools/bench-memory/README.md). In particular:

- Grid memory includes allocating/filling the input Vec; its tiny construction
  **timer** excludes that allocation. These are different declared scopes.
- Worldgen memory includes one generated map + hash, not JSON serialization.
- Person includes stable IDs, current components, ECS metadata and runtime
  startup. Cold per-person quotients are 2,785.28 B (100) and 589.824 B (1,000);
  prepared quotients are 2,621.44 B and 573.44 B. These average process
  increments include fixed overhead and **are not individual Person sizes**.
  Runtime-map-only bytes remain unmeasured (optional in CHRON-021).
- Needs includes its Vec and all 8,760 advances per person-year, not a full NPC
  kernel or a 10-year run.
- Sites operate on 20 prepared sites and release the work-copy after 10,000
  updates. Zero extra resident pages does not mean zero allocations.
- Path cold costs include map/BFS/case preparation and diagnostic expansion
  searches; operation costs cover one selected A*. Trivial paths require no
  additional pages here. Do not attribute the cold preparation peak to one A*.
- Candidate memory retains all 861/9,060 trace objects. Utility memory retains
  100/1,000 full Selection objects, with ten candidates each. Their streaming
  timing loops have intentionally different result lifetimes.
- The largest observed cold increment is 6,799,360 B; largest process peak in
  these samples is 8,388,608 B. Neither is the memory use of a full game.

## Unchanged timing paths: follow-up evidence

Exact aggregate stdout for all eight examples, plus both diagnostic repeats:
[rem-008a-timing.jsonl](data/rem-008a-timing.jsonl).
Original pre-tool evidence remains in each CHRON report; the new file is the
authoritative follow-up timing series. No individual timing samples are claimed
beyond the harness's printed min/median/max.

| Workload | Original REM-008 median | Follow-up primary median |
|---|---:|---:|
| Grid build / scan | 41 ns / 11,291 ns | 42 ns / 13,375 ns |
| Worldgen seeds 0 / 1 / 42 | 594,375 / 339,791 / 291,083 ns | 579,459 / 334,541 / 291,000 ns |
| Person 100 / 1,000 | 41,125 / 148,750 ns | 15,584 / 68,250 ns |
| Needs 100 / 1,000 person-years | 1,016,666 / 5,367,584 ns | 1,021,708 / 5,420,209 ns |
| Site query / work update | 26 / 18 ns | 26 / 18 ns |
| Candidate enumeration 100 / 1,000 | 40,003,583 / 364,870,208 ns | 39,697,000 / 363,050,833 ns |
| Candidate traces 100 / 1,000 | 59,642,792 / 554,274,333 ns | 59,060,334 / 551,229,167 ns |
| Utility 100, epsilon 0 / 25 | 26,539,417 / 26,490,292 ns | 25,908,167 / 25,990,333 ns |
| Utility 1,000, epsilon 0 / 25 | 239,206,125 / 239,273,000 ns | 235,583,166 / 236,847,708 ns |

Path follow-up medians in case order trivial/short/medium/long/unreachable/
node_budget/path_budget: 42 / 4,875 / 8,042 / 29,667 / 934,333 / 13,625 /
268,125 ns. Paths, expansions, JSON sizes and deterministic checksums match the
original fixtures.

The primary grid scan is 18.5% slower than the historical run, but a second
unchanged-binary ten-sample run measured 6,458 ns (and build 0 ns), demonstrating
large short-benchmark scheduling/frequency/timer variation. The Person repeat
measured 15,416/66,625 ns. No source optimization occurred and neither the
apparent Person speedup nor an isolated grid slowdown is attributed to this
tool. This evidence does not establish a stable regression or improvement;
sub-microsecond construction timing is not a useful optimization gate.
The larger candidate/Utility workloads remain near the earlier baseline.

## File ownership and independent review

Parent implemented the native tool, tests, manifest/lock edges, CI addition,
task/ADR and reports. Luna dispatch fitness was READY after the parent froze
the adapter contract and verified the existing fixture/helper/checksum sources.

- /root/rss_world_adapters: four sim-world examples.
- /root/rss_ai_core_adapters: three sim-ai examples + Person example.
- Requested model for both: gpt-5.6-luna, medium, no-history fork. The dispatcher
  exposed agent identities, not backend routing verification.
- One bounded rework per worker addressed new-test placement/Clippy and missing
  runtime checksum assertions. Parent inspected actual diff, verified goldens
  against earlier reports, ran combined checks and all memory/timing cases.
- No overlapping writes, recursive workers, OpenCode, external model API,
  sidebar task creation or claims of measured quota savings.

Final source/lock hashes (SHA-256):

| File | SHA-256 |
|---|---|
| Cargo.lock | fc9e78b4a732ca278bfcfdce4202adb8c0dc5d3aff42395a5ad2a855187813f0 |
| tools/bench-memory/Cargo.toml | da04b810571a23228616b2559a9ce31e0df842b02107e8e42dce8b5cd4e83091 |
| tools/bench-memory/src/main.rs | bee21bec110c9a34dbbc9ab0863bde5f04d4515dabc304160a965656d89e514c |
| tools/bench-memory/src/rss.rs | 867f9039da0b7a4e382bb359c1aadb231dabf23a07cf488d20ccebfd631ebdf0 |
| tools/bench-memory/tests/cli.rs | fb58722d4c6b4e6f8900c5efd6fec327863db736ea95035753f55c7d75106acf |
| crates/sim-world/examples/grid_bench.rs | a2a02156f0092ac49c866a620d38e2b363892cb0cd8d755d0015a006356b6cc3 |
| crates/sim-world/examples/worldgen_bench.rs | 9c6433834944226c867da150ccaac1678489eb201b86bfb85c8b5bdc0a303bfa |
| crates/sim-world/examples/site_bench.rs | 8de5c772006fed8245deee772052ee954d6e0598907957c3a898c7bcc3311fa6 |
| crates/sim-world/examples/pathfinding_bench.rs | e337bcd70f694fc98cc94ef1ac73017e4e6d0ff4aa29dfa43d741b8f6a53b431 |
| crates/sim-core/examples/person_spawn_bench.rs | 9a26545fe757bd8ebd629530da758ff3a63843fb7730e2ca23be5408323d77ec |
| crates/sim-ai/examples/needs_bench.rs | e967d94ba33d59d156517c6ed40769f6e8fdffec73306dd17364f835720ffcad |
| crates/sim-ai/examples/utility_ai_bench.rs | 8af29f5b716c8207cc0dddb67567a0bf5c9708b9df8e297d3456c48f372dca4b |
| crates/sim-ai/examples/utility_score_bench.rs | b1ca63b1d9ac3d4fa044ffcc77a016ff3431e0332ae5cdd8810ca9420482f909 |

Production remediation hashes in PERFORMANCE.md are unchanged. MASTER_SPEC.md
remains a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea.
Cargo.lock adds only the local tool package and its edges; every third-party
package/version/checksum is unchanged.

## Final gates, limitations and sensitive changes

Parent final checks actually run:

- `./tools/ci-rust.sh`: Master Spec hash, workspace fmt, warnings-denied
  all-target/all-feature Clippy, **203 unit/integration tests, zero failed or
  ignored**, Rust 1.95 MSRV and all seven existing release smoke commands passed.
  The new tool runs 19 unit/adapter tests and 7 native CLI tests; 14 adapter
  tests also execute as their original examples. Original domain tests remain.
- `cargo test --workspace --doc`: **2 passed**, including the public-API
  compile-fail test. `RUSTDOCFLAGS='-D warnings' cargo doc --workspace
  --all-features --no-deps`: passed.
- `cargo test -p palimpsest-bench-memory --release --test cli --locked`:
  **7 passed**, including real transient release and prior-peak contamination.
- `cargo metadata --no-deps --format-version 1 --locked --offline` and the
  normal `cargo tree` for sim-world/sim-ai: graph verified. sim-world normal
  dependencies remain `{serde}`, sim-ai remains `{sim-time, sim-world, serde}`.
  No domain crate depends on the measurement tool or outward on the bridge.
- `cargo fmt --all -- --check`, `git diff --check`: passed. Source/lock hashes
  verified; the eight example diffs contain only additions. Original simulation
  source hashes and MASTER_SPEC.md match the pre-extension baseline.
- Workflow YAML was parsed with Ruby; the two existing required job names and
  new native-test command were checked. `actionlint` is not installed; no full
  actionlint or remote Actions run is claimed.

Definition of Done: all required per-case cold and prepared-operation peak
measurements are present with kernel provenance; tests and original timings
are preserved; every affected task report links the new evidence. No remaining
REM-008A implementation/measurement blocker. Future scope/acceptance boundaries
remain unchanged.

Godot runtime/scene files were not changed and Godot was not relaunched in this
follow-up; the previous REM-008 integration evidence remains historical. The
exact newly added macOS CI command passed locally. The Linux-only unsupported
platform branch was reviewed but not executed on Linux in this local session.

Sensitive boundary: a small, explicit native unsafe exception now exists only
in the measurement binary (Mach reads and fixed-size diagnostic mappings).
Every production crate retains unsafe-forbid. The native integration tests are
added to the existing macOS CI job; Linux workspace tests alone cannot execute
Mach. Existing steps/check names/permissions/timeouts are unchanged. This is a
local workflow edit, not a remotely executed green CI or protection-setting
change.

Limits: macOS measurement only; three cold RSS samples, OS page granularity,
allocator reuse and ordinary OS memory pressure; first-use/fixture overhead
must not be called steady-state object size. No full-world memory, runtime-map
isolation, Godot rendering, 100-NPC/10-year or Phase 1 completion claim. The
~26 ms 100-person selection round remains a future kernel cadence concern,
not permission to optimize or implement that kernel now.
