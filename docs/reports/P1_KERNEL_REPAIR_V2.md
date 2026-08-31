# P1 Kernel Repair V2 — Completion Report

- Date: 2026-08-31; plan: [P1-KERNEL-REPAIR-V2, r1](../tasks/P1_KERNEL_REPAIR_V2.md).
- Status: **Implemented / locally verified**. R2-01–06 are complete; this is
  repair verification, not owner acceptance of all Phase1 or hosted CI.
- Authority: the owner requested a bounded plan and subagent implementation;
  [ADR-0025](../adr/ADR-0025-kernel-repair-completion.md) records the decisions.
- Scope ends at CHRON-027–029 repairs, reproducible measurements and context
  routing. No CHRON-030+, Phase 2, remote mutation, commit or push.

## Repair evidence map

| Finding | Correction | Independent regression / evidence |
|---|---|---|
| Start/cancel upper-bound rejection mutated state | Precompute Needs, continuation/check deadlines and capacity before commit | `action_repair_v2`: upper-bound start/cancel; original Idle completes once after rejection |
| Movement/arrival allowed backward cancellation | Separate per-person last successful commit timestamp from lazy Needs baseline | movement t1/cancel t0 and work-arrival regressions; original future trajectory preserved |
| Faulted metrics/next_due/sites leaked partial state | Cache complete-boundary action metrics, label lifecycle/failure instant, guard dynamic reads; no projected-Needs fallback | `kernel_repair_v2` real overflow and private unit regression where WorkCounter actually mutates before another person's failure |
| Per-call events counted retained records only | Return generated upstream delta, including rotated events | 4,095/4,096/4,097 edges; capacity1/default, drain/segmentation accounting; independent ordered FNV oracle |
| Independent DTO decode bypassed validation | Private wire types call shared TerrainBatch/PersonRender validators; Moving Idle rejected | `render_repair_v2` standalone/root negative and valid combination matrices; existing schema2 tests retained |
| Benchmark protocol incomplete | Strict CLI, complete raw samples, one Duration per sample, upper median, full run counters/truth and call-boundary queue observations | parser/precision tests, executable CLI checks and raw evidence below |
| Render RSS observed released bytes / wrong control interval | Matched read-only control inside interval; kernel, DTO and bytes borrowed beyond second callback | adapter two-callback, truth-equivalence, zero-person/null and native CLI tests; native RSS algorithm unchanged |

Existing tests were preserved. New tests complement rather than replace the
V1 cases. Token exhaustion remains covered by the existing scheduler/action
preflight tests; an EventId-exhausted cancellation regression was added without
introducing a production fault-injection API.

## Delegation and review

Three bounded native subagents were requested with `gpt-5.6-luna`, medium
reasoning and no full-history fork: actions, DTO validation and benchmark
tooling. File ownership and fixed contracts were assessed before dispatch.
The requested model is recorded here; the tool did not expose an independently
verifiable backend model identity or per-agent billed token usage.

Actions received one test-focused rework; benchmark tooling received one
kernel-only rework. The parent then took over the remaining incomplete tooling
and test details, integrated the `sites()` Result boundary, and ran independent
verification. Kernel fault publication, ADR decisions, final measurement and
acceptance remained parent-owned. No OpenCode/provider configuration changed.

## Verification and measurements

### Commands actually run

| Command / check | Result | Durable evidence |
|---|---|---|
| `./tools/ci-rust.sh` | Pass: fmt, workspace Clippy `-D warnings`, 330 debug tests (0 failed/ignored), MSRV1.95 check, seven Phase0 smoke benchmarks | [full log](data/kfix-v2-ci-rust.txt) |
| `cargo test --release --locked --workspace --all-targets --all-features` | 330 passed, 0 failed/ignored | [release log](data/kfix-v2-release.txt) |
| `cargo test --locked --workspace --doc` | 2 passed; all11 crates checked | [doctest log](data/kfix-v2-doctest.txt) |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` | Pass | [rustdoc log](data/kfix-v2-rustdoc.txt) |
| `cargo metadata --locked --no-deps --format-version 1`; `cargo tree --locked --workspace --edges normal` | Exact local normal edges reviewed: AI→time/world; Core→AI/world/domain crates; no Core→Godot/storage/bench-memory edge | [metadata](data/kfix-v2-metadata.json), [tree](data/kfix-v2-tree.txt) |
| `./tools/ci-godot.sh` | Godot4.7.2 loads godot-rust, no script/error/crash marker | [Godot log](data/kfix-v2-godot.txt) |
| Release executable negative CLI checks | 23 unknown/duplicate/missing/noninteger/zero-sample/zero-action-or-kernel-person cases exit2 with empty stdout; zero-person Render succeeds with null ratio | Same binaries captured in environment manifest |
| `cargo fmt --all -- --check`; targeted core/memory all-target Clippy; release example + memory builds | Pass after the output-label correction below | Final binaries captured in environment manifest |

The suite counts include benchmark adapter tests embedded in both examples and
the memory tool; they are test executions, not330 unique domain properties.
Core library has41 tests; the old17 kernel-repair tests, original integration
suites and10 Scheduler tests remain intact. New focused suites add6 action,
4 Kernel,3 DTO and6 benchmark-protocol executions, plus module/adapter tests.

During the first collection, parent review found an inaccurate kernel units
label implying initialization decisions were counted. The actual counters
start at advancement. Collection was interrupted during the first year warmup
(SIGINT to that exact benchmark child); already written output, source identity
and failed command record are retained as `data/kfix-v2-interrupted-*`.
The correction changed only a units string and explanatory comment, not
simulation logic or assertions. The targeted checks/rebuild above passed,
then all final timing/RSS cases were restarted under a new frozen identity.
This is a disclosed source correction, not failed/slow-sample filtering. The
full suite logs precede this metadata-only change; final measurements use the
corrected binary. No repeat of unchanged expensive full tests was required.

### Final-source measurement protocol

Reference host: Apple M5,10 CPUs,17,179,869,184 bytes (16GiB), macOS26.6.2;
rustc/cargo1.98.0. [Environment](data/kfix-v2-environment.json) records dirty
HEAD,74 source hashes and four release binary hashes. The collector checks
source/binary identity before and after every command. Measurements run
sequentially without concurrent agent builds/tests. OS/app activity, thermal
conditions and CPU frequency remain uncontrolled.

Run: `ruby tools/collect-kfix-v2.rb all`. It invokes the prebuilt release
examples, two full warmups then ten timing samples, and five RSS cases with
three fresh processes each. The one-day kernel smoke is explicitly1 sample
without warmup, not a formal timing distribution. Kernel includes advances
and final truth validation in its interval; initial setup/start is excluded.
Render build and serialize are timed separately; validation is outside those
timers but inside the RSS operation. Kernel queue maximum is observed at
advance-call boundaries, not a claimed per-item or per-round peak.

Final collection finished at12:12:28+08:00 on2026-08-31. All10 final measurement
commands exited0. [Command log](data/kfix-v2-commands.jsonl) and
[independent validation](data/kfix-v2-validation.json) confirm40 formal timing
samples plus1 smoke sample,15 distinct cold PIDs, consistent truth/summary
values and unchanged source/binary hashes. No V1 result substitutes for this evidence.

| Timing | Fixture / horizon | Min / upper median / max wall time | Committed work / result |
|---|---|---|---|
| Action100 | seed25025, strided connected region,172800s | 79.627 / 80.761 / 85.860ms | 27931 transitions |
| Action1000 | same fixture/horizon | 735.942 / 819.387 / 850.722ms | 280354 transitions |
| Kernel one-day smoke | seed42/default sites,100 colocated,86400s | 412.293ms (one sample; not distribution) | 635 rounds;65900 transitions;209560 sim-s/wall-s |
| Kernel one-year | same,31536000s | 155.307 / 155.473 / 222.367s | 193515 rounds;20247700 transitions;672200 decisions/events;202840 sim-s/wall-s |
| Render100 schema2 | seed42, row-major spawn, kernel at600s | build15.667 / 17.750 / 25.500µs; serialize186.459 / 189.208 / 282.584µs | 153014bytes;152.93 persons-section bytes/person |

Timing raw: [Action](data/kfix-v2-action-timing.jsonl),
[Kernel](data/kfix-v2-kernel-timing.jsonl), and
[Render](data/kfix-v2-render-timing.jsonl). They retain stats, counts,
event digest, final checksum, queue/stale nodes and per-wall-second rates.
The unchanged memory-adapter golden checksums still pass. V1 action aggregates
were84.539/891.686ms but lacked full raw series and used an inconsistent median
protocol: these observations are not a controlled before/after speedup claim.
Cross-sample equality is a determinism check, not a replacement for the
independent correctness regressions. The one-year max outlier is retained;
no rerun or thermal outlier deletion was performed. The restored colocated
Kernel dataset differs from V1 BFS-spread spawn and is a new baseline, not a
claimed speedup. Render and Kernel intentionally use different spawn layouts.

### Native RSS — bytes, three fresh processes per row

| Case | Cold min / upper median / max | Prepared min / upper median / max |
|---|---|---|
| Action100 | 3702784 / 3702784 / 3719168 | 3178496 / 3194880 / 3211264 |
| Action1000 | 7684096 / 7733248 / 8011776 | 6881280 / 6930432 / 7225344 |
| Kernel100 one-year | 7258112 / 7274496 / 7307264 | 5849088 / 5881856 / 5947392 |
| Render control100 | 1589248 / 1622016 / 1785856 | 65536 / 65536 / 81920 |
| Render snapshot100 | 1769472 / 1851392 / 1900544 | 212992 / 229376 / 229376 |

[Raw RSS intervals](data/kfix-v2-memory.jsonl) include baseline/end current RSS,
lifetime peak, proof, increment, PID and checksum. All15 cold and15 prepared
intervals are proved; none are ambiguous or retried. Action goldens match the
original adapters; annual Kernel checksum matches the timing run; Render
control/snapshot have identical underlying truth checksums.

Cold includes setup, first-touch and allocator/validation overhead; prepared
excludes fixture setup but includes operation assertions. Snapshot validation
allocations are included and kernel/DTO/bytes remain live beyond the second
observation. Control/snapshot differences are not a pure heap-allocation
measurement. V1 retention/control errors invalidate a direct RSS comparison.

## Task closure and file scope

| Task | Done evidence | Benchmark |
|---|---|---|
| R2-01 actions | Six focused regressions, added EventId exhaustion unit, unchanged original suites; parent review | Final Action100/1000 + annual Kernel |
| R2-02 Kernel | Four focused regressions plus real partial-commit fault unit; old kernel suites | Final annual Kernel + RSS |
| R2-03 DTO | Three new matrix regressions, old schema2 suite, benchmark roundtrip/truth checks | Render build/serialize/RSS |
| R2-04 tooling | Strict CLI/median/precision/zero-person/adapter tests, independent raw validator | R2-05 |
| R2-05 measurement | Frozen74 source/four binary hashes, all10 commands exit0,41 timing/15 cold samples validated | Tables/raw above |
| R2-06 closure/routing | Local full gates, Master hash, links/scope checks, corrected historical headers, compact current/index entries | Reuses R2-05; N/A for document edits |

The [pre-repair manifest](data/kfix-v2-preexisting-sha256.json) preserves the
216-file dirty-tree baseline. [Final scope manifest](data/kfix-v2-scope.json)
lists actual before/after changes and additions (excluding its own hash).
The existing dirty HEAD diff is not presented as all authored by this repair.
No pre-existing file was deleted; old raw data, Master, native RSS, CI scripts,
dependency manifests/lockfile and unrelated domain code retain baseline hashes.

Known limitations are listed below. There is no remaining blocker within this
repair plan. CHRON-030+ was not started and no commit/push/remote change occurred.

## Context routing

[CURRENT_PROGRESS](../CURRENT_PROGRESS.md) and [TASK_INDEX](../TASK_INDEX.md)
are the small current entrypoints. Closed plans remain available for contract
changes, regression diagnosis, evidence/approval checks or explicit requests;
their full text is not recursively loaded by default. New task packets link
only relevant contracts and evidence. The four mandatory specification files
remain required; summaries cannot override them.

This reduces avoidable repeated historical reads, not existing chat history or
all future context costs. A new session can start with the compact entrypoints
and required specifications. No token-saving percentage is claimed.

## Sensitive changes and limits

- `WorldKernel::sites()` and `next_due()` now return Result; new metrics
  lifecycle/failure fields and explicit Needs projection errors affect callers.
  Faulted kernels are diagnosable but are not rollback/recovery implementations.
- Action rejection is stricter about time monotonicity; DTO decoding rejects
  inconsistent data. Schema stays2; this is not a save-format migration.
- Kernel measurement restores the planned colocated fixture. Old BFS-spread
  V1 timing is not a comparable baseline and cannot demonstrate optimization.
- RSS is native resident peak increment, not object heap size, Apple physical
  footprint or full Core+Client usage. Prepared intervals may correctly be null.
- The 1,024 budget is rounds, not items or a wall-time guarantee. No 100-NPC /
  ten-year, Godot FPS or Phase1 memory-budget acceptance follows from this work.
- The working tree was already dirty/untracked. Hosted required checks and
  remote branch protection are not certified by local test results.
- The memory tool intentionally embeds three standalone examples, each with
  its private protocol helper. Its documented `clippy::duplicate_mod` allowance
  covers that embedding pattern; no production-core lint or test was disabled.
