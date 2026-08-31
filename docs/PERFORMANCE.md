# Palimpsest Performance Baseline

`MASTER_SPEC.md` defines the M5 16GB performance contract. This document records methods and measured results; it cannot relax product budgets.

## Reference Machine

- Apple M5, 10 CPU cores
- 16 GB unified memory
- arm64 macOS
- Rust 1.98.0 stable

Machine serial numbers and other device identifiers are intentionally excluded.

## Measurement Rules

- Use release builds for throughput and latency claims.
- Record workload, sample count, warm-up, exact command, and relevant dependency versions.
- Keep correctness assertions enabled in benchmark harnesses.
- Report median and limitations; do not treat one noisy CI run as a performance gate.
- Scale entity workloads through 100, 1K, 3K, 5K, and 10K where applicable.
- Preserve identity, causality, history fidelity, and tests during optimization.

## Results

Task-specific raw results live under `docs/reports/`. Architecture-wide conclusions belong only in `docs/reports/ARCHITECTURE_SPIKE_V1.md` after all Phase 0 measurements complete.

- CHRON-006 Scheduler: `docs/reports/CHRON-006_SCHEDULER_BASELINE.md`
- CHRON-008 10K ECS dummy entities: `docs/reports/CHRON-008_10K_DUMMY_BENCHMARK.md`
- CHRON-016 structured-event throughput: `docs/reports/CHRON-016_EVENT_THROUGHPUT.md`
- CHRON-013 SQLite Event Store: `docs/reports/CHRON-013_SQLITE_EVENT_STORE.md`
- CHRON-012 Snapshot prototype: `docs/reports/CHRON-012_SNAPSHOT_PROTOTYPE.md`
- CHRON-003 Godot-Rust bridge: `docs/reports/CHRON-003_GODOT_RUST_BRIDGE.md`
- CHRON-011 128×128 Tile renderer: `docs/reports/CHRON-011_TILE_RENDERER.md`

## Phase 1 Remediation Measurement Method — 2026-08-30

This is evidence for existing CHRON-019..026 components, not completion of
Phase 1 or its 100-NPC/10-year kernel gate. The original run left the stronger
peak-incremental RSS requirement open. The owner-approved REM-008A extension
below supplies a separate kernel-accounted series; historical endpoint deltas
are not relabeled and no budget is relaxed.

Reference environment verified locally:

- Apple M5, 10 logical cores; hw.memsize = 17,179,869,184 B (16 GiB).
- macOS 26.6.2, arm64; local measurement on 2026-08-30, approximately 13:18–13:24 +08:00.
- rustc 1.98.0 (88d9e12ae 2026-08-18); cargo 1.98.0 (797e8a9bc 2026-08-05).
- Locked dependencies: bevy_ecs 0.19.1, serde 1.0.229, serde_json 1.0.151;
  godot Rust bindings 0.5.5, engine 4.7.2 stable official, gda 0.12.0.
- Base commit e5b0aeb676372a123dd8c27190e94b6a606d498c plus **uncommitted local
  remediation**. These results are not remote CI results or a new commit.
- Cargo.lock SHA-256: `874f6bba386219e403591e53b36b1b3af39e144aaf253b5b50734de074b413a1`.
- Read-only MASTER_SPEC.md SHA-256 remains
  `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`.

Remediated source SHA-256 identifiers (relative to the repository root):

| File | SHA-256 |
|---|---|
| crates/sim-ai/src/action.rs | `8f719e509789d44adeba32c1100a56d2033ea9129f765e8a827182850677f60e` |
| crates/sim-ai/src/utility.rs | `44e0b1a46cb0ee6041d6c4383243cf495e0ce1a50fee945c080a73e5dd53d879` |
| crates/sim-ai/src/trace.rs | `0fd8ae69db0d4051edb4704e5db0075fc9a09460f9fc0cec0110e3758cd69246` |
| crates/sim-ai/src/lib.rs | `af985b3ae7815f995ec7927b3469b7c07f5d8f78efe24467493aaeddeb3e9635` |
| crates/sim-core/src/person.rs | `ce73a564c6cdea3a45c14547041e3658c5ef6d696eb61bb61d0206a76663e472` |

### Commands and correctness

Run from the repository root; final benchmarks were run sequentially from
prebuilt release executables, after tests and builds finished:

```sh
./tools/ci-rust.sh
cargo test --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
./tools/ci-godot.sh
gda info --json
gda script validate --all --project /Users/gabrielmu/Documents/Palimpsest/apps/macos-godot --json
cargo metadata --no-deps --format-version 1 --offline
cargo tree -p palimpsest-sim-world --edges normal --offline
cargo tree -p palimpsest-sim-ai --edges normal --offline
cargo build --release -p palimpsest-sim-world -p palimpsest-sim-ai -p palimpsest-sim-core --examples
/usr/bin/time -l target/release/examples/grid_bench 10 2
/usr/bin/time -l target/release/examples/worldgen_bench 10 2
/usr/bin/time -l target/release/examples/person_spawn_bench 10 2
/usr/bin/time -l target/release/examples/needs_bench 10 2
/usr/bin/time -l target/release/examples/site_bench 10 2
/usr/bin/time -l target/release/examples/pathfinding_bench
/usr/bin/time -l target/release/examples/utility_ai_bench 10 2
/usr/bin/time -l target/release/examples/utility_score_bench 10 2
git diff --check
```

All commands passed. ci-rust.sh includes workspace formatting, warnings-denied
Clippy, 163 unit/integration tests (zero failed/ignored), Rust 1.95 MSRV check,
and seven existing release smoke checks. The separate doctest run passed two
tests, including forbidden runtime-handle access. Rustdoc denies warnings.
Godot-rust initialized API 4.7 / runtime 4.7.2 with strict safeguards; existing
headless scene smoke emitted no errors. gda returned valid=true for main.gd,
metrics_overlay.gd and tile_renderer.gd, each with no diagnostics and the
correct project_root. No Godot scene, script, bridge contract or CI workflow
was changed. Metadata/tree review confirms the documented inward graph.

### Interpretation limits

- Every final timing case uses ten samples after two warmups; the pathfinding
  harness hardcodes these counts. Median is the sorted upper-middle sample,
  not the arithmetic average of two middle values.
- Exact aggregate JSON output is embedded in each report. The unchanged
  harnesses do not print individual samples; min/median/max is not a raw
  per-sample dataset. No confidence interval is claimed.
- `rss_delta_bytes` is a ps before/after retained-state sample after warmup,
  with KiB converted to bytes. Allocator reuse and sampling noise can yield
  zero; it is not object size, allocation count or a peak.
- `/usr/bin/time -l` adds whole-command max resident set size and peak memory
  footprint. These cover setup and every scale/seed in that invocation, not
  a per-operation or per-scale incremental peak. They do **not** close the
  stronger peak-incremental requirement in CHRON-019/020/022..026.
- At this historical checkpoint, accurate interval-scoped peak capture needed
  tooling beyond REM-008's report-only whitelist. The owner subsequently
  approved REM-008A; see the new method/results below. No acceptance waiver
  was used, and these historical ps/time fields remain non-equivalent.
- No competing agent build/test/benchmark was scheduled during final timings.
  Ordinary OS/app activity, CPU frequency and thermal conditions were not
  controlled. Differences of a few percent are not optimization evidence.
- Utility timing includes reachability checks and full trace construction;
  candidate-trace timing includes candidate enumeration. Do not sum unlike
  fixture timings or interpret these as pure arithmetic costs.
- Existing Phase 0 integration was checked, but rendering FPS, a Phase 1
  Godot Micro World and the 100-NPC/10-year gate were not run or claimed.

### Authoritative per-task results

Use the final REM-008 section where a report also retains earlier before/after
smoke evidence. Numeric results are not duplicated here.

- [CHRON-018 boundary review](reports/CHRON-018_WORKSPACE_BOUNDARIES.md)
- [CHRON-019 local grid](reports/CHRON-019_LOCAL_GRID.md)
- [CHRON-020 world generation](reports/CHRON-020_WORLDGEN.md)
- [CHRON-021 person runtime](reports/CHRON-021_PERSON_RUNTIME.md)
- [CHRON-022 Needs](reports/CHRON-022_NEEDS.md)
- [CHRON-023 activity sites](reports/CHRON-023_ACTIVITY_SITES.md)
- [CHRON-024 pathfinding](reports/CHRON-024_PATHFINDING.md)
- [CHRON-025 candidates/traces](reports/CHRON-025_ACTION_TRACE.md)
- [CHRON-026 Utility scoring](reports/CHRON-026_UTILITY_SCORING.md)
- [CHRON-027 action execution](reports/CHRON-027_ACTION_STATE_MACHINE.md)
- [CHRON-028 kernel orchestration](reports/CHRON-028_KERNEL.md)
- [CHRON-029 render snapshot](reports/CHRON-029_RENDER_SNAPSHOT.md)
- [CHRON-030 simulation worker](reports/CHRON-030_SIMULATION_WORKER.md)

CHRON-028/029 originally recorded pilot measurements. Final two-warm-up,
ten-sample M5 runs and native peak-incremental RSS are now recorded by
[P1 Kernel Repair V2](reports/P1_KERNEL_REPAIR_V2.md), below; they are no longer
deferred to CHRON-033. Scale, ten-year and Core+Client validation remain separate.

## REM-008A — Kernel Peak RSS Follow-up, 2026-08-30

The owner explicitly included the tooling extension. Method and safety
boundary: [ADR-0020](adr/ADR-0020-benchmark-memory-measurement.md).
Authoritative results, commands, source hashes and limits:
[REM-008A memory report](reports/REM-008A_MEMORY.md).
Raw samples: [22 cases × three independent processes](reports/data/rem-008a-memory.jsonl).

This method reads macOS current RSS and kernel lifetime RSS high-water, then
proves the peak occurred inside the chosen interval before subtracting the
starting current RSS. It reports both cold fixture-plus-operation and
prepared-operation intervals. Ambiguous earlier peaks are rejected/labeled;
there is no polling peak approximation, allocation-byte substitution or
subtraction of one lifetime maximum from another. Three cold memory samples
are separate from the unchanged ten post-warmup timing samples.

All 66 final samples have proved cold **and prepared-operation** increments;
none required discarding/retrying or a measurement exception. Page size is
16,384 B. Zero operation increments (sites, trivial path) reflect already
resident fixture/allocator pages, not zero object memory. The transient probe
records 67,108,864 B even when ending RSS returns to baseline; the prior-peak
probe correctly returns an unavailable prepared value.

Only the new outward-only tool uses the already-locked libc 0.2.189; Cargo.lock
adds its local package edges without third-party version updates. Core source
hashes above are unchanged. The follow-up report supplies the new lock/tool
hashes and local gates; old remote CI does not certify uncommitted work.

## P1-KERNEL-REPAIR — KFIX-007 final-source evidence, 2026-08-31

> Historical V1 data, **not fully accepted**: independent review found missing
> Action raw samples, an inconsistent median algorithm, last-call rather than
> total kernel rounds, and an invalid retained-object interval in the render
> adapter. The V1 files remain unchanged. Final replacement evidence is owned by
> [repair V2](tasks/P1_KERNEL_REPAIR_V2.md); do not infer all-green acceptance
> from the historical table below.

Targeted repair measurement (ADR-0024; plan P1-KERNEL-REPAIR). Correctness
assertions stay enabled in every timing sample; timing used two full workload
warm-ups and ten samples, and RSS used three independent cold subprocesses per
case. Raw data: `reports/data/kfix-v1-{action,kernel,render}-timing.jsonl` and
`reports/data/kfix-v1-memory.jsonl`.

| Measurement | Fixture / horizon | Result (median) |
|---|---|---|
| Action 100 | seed 25,025 reference sites, 172,800 s | 84.539 ms/run |
| Action 1,000 | same, 172,800 s | 891.686 ms/run |
| Kernel 1-day (smoke) | seed 42 default sites, 86,400 s | 198,187 sim-s/wall-s |
| Kernel 100-person 1-year | seed 42, 31,536,000 s | 183,671 sim-s/wall-s |
| Render 100 (schema 2) | seed 42, kernel at 600 s | build 9.94 µs; 153,014 bytes |
| Memory cold (3 procs) | action-100 / action-1000 / kernel-100-year / render-control-100 / render-snapshot-100 | 3,506,176 / 7,815,168 / 6,324,224 / 1,622,016 / 1,703,936 B |

This is a correctness-throughput record, not a Phase 1 acceptance: the
100-NPC/10-year gate remains CHRON-032, and no 3/5/7 GB or 60 FPS claim is made.
The digest is a deterministic, non-cryptographic FNV-1a-64 stream.

## P1-KERNEL-REPAIR-V2 — Verified final-source evidence, 2026-08-31

Authoritative [report](reports/P1_KERNEL_REPAIR_V2.md),
[source/binary identity](reports/data/kfix-v2-environment.json), and
[raw validation](reports/data/kfix-v2-validation.json). Host: Apple M5/16GiB,
macOS26.6.2, Rust1.98 release. Forty formal timing samples plus one day-smoke
sample; five RSS cases × three independent cold processes. All cold/prepared
proofs valid. No failed/slow samples filtered or retried in the final run.
An interrupted earlier collection for a units-label correction is preserved
and explicitly excluded; see the report. No concurrent builds/tests during timing.

| Measurement | Upper median |
|---|---|
| Action100 / Action1000,172800s | 80.761 / 819.387ms |
| Kernel100,31536000s | 155.473s;202840 sim-s/wall-s |
| Kernel full-run work | 193515 rounds;20247700 transitions;672200 decisions/events;max queue observed200 |
| Render100 at600s | build17.750µs;serialize189.208µs;153014bytes |
| Cold RSS increments: Action100 /1000 | 3702784 /7733248bytes |
| Cold RSS: Kernel100-year | 7274496bytes |
| Cold RSS: Render control /snapshot | 1622016 /1851392bytes |

Kernel restored the approved colocated-first-walkable fixture; V1 BFS-spread
results are not comparable optimization evidence. V1 Action raw/median and
Render retention/control defects also prevent treating old/new aggregates as
a controlled performance comparison. The 222.367s annual maximum is retained.
Detailed min/median/max, prepared intervals, raw counters and limits are in the
report. Queue maximum is at advance-call boundaries, not a per-item peak;
budget1024 is rounds, not a latency guarantee. RSS includes validation and is
not heap size/physical footprint. No ten-year, FPS or Core+Client budget claim.

## CHRON-030 — Simulation Worker, 2026-08-31

Authoritative [report](reports/CHRON-030_SIMULATION_WORKER.md); raw timing
[data](reports/data/chron-030-worker-bench.json) and
[memory data](reports/data/chron-030-worker-memory.jsonl). Same host and
protocol: release, two warm-ups plus ten samples, 100 persons, seed 42,
86,400 s, colocated fixture; three cold processes for RSS (ADR-0020).

| Measurement | Upper median |
|---|---|
| Direct-kernel control wall | 409.901 ms |
| Worker submit→ack wall | 417.379 ms (overhead 1.016×) |
| Submit→publication visible | ≤ ack (publication precedes ack) |
| Command throughput (paused no-op steps) | 4,757,259 commands/s |
| Max observed queue depth | 64 (capacity) |
| Cold RSS increment, worker-100-day | 5,259,264 bytes |
| Prepared→acked-day RSS increment | 3,670,016 bytes |

Worker and direct control produced the identical truth checksum in every
sample. Pacing diagnostics (short wall windows): numeric multipliers track
their nominal factor in order; MAX reached ≈181,784 sim-s/wall-s against the
direct kernel's ≈202,840 year rate. This is a worker-overhead record, not the
100-NPC/10-year gate (CHRON-032) and not a 3/5/7 GB or 60 FPS claim.

## CHRON-031 — Godot Micro World Presentation, 2026-08-31

Authoritative [report](reports/CHRON-031_GODOT_MICRO_WORLD.md); raw capture
[data](reports/data/chron-031-frames.json) and base-terrain
[variant](reports/data/chron-031-frames-minimal.json). Godot 4.7.2
Forward+/Metal windowed on the M5 16 GB host with a release GDExtension; 120
warm-up frames discarded, 300 consecutive measured frames, world driven at
100× so all 100 persons move.

| Metric | Full scene | Base terrain only |
|---|---|---|
| FPS min / mean / p95 | 60.0 / 60.0 / 60.0 | 60.0 / 60.0 / 60.0 |
| Frame time mean / p95 / max | 16.667 ms (vsync) | 16.667 ms (vsync) |
| Draw calls min / mean / p95 | 19 / 19.0 / 19 | 1 / 1.0 / 1 |
| Video memory p95 | 57,245,696 bytes | 32,161,792 bytes |

VSync-capped sustainment, not uncapped headroom. The 100-person MultiMesh adds
exactly one draw call over base terrain; the remaining 17 are site markers and
UI. This satisfies the 100-person 60 FPS presentation target; the 200-year and
3/5/7 GB budget gates remain separate (CHRON-032/033).

## CHRON-032 — 10-year headless chaos evidence, 2026-08-31

Correctness/no-crash gate (ADR-0027), not a budget claim (CHRON-033/036 hold
3/5/7 GB and 60 FPS). Fixture: seed 42, 100 persons, `315_360_000` s (10
365-day years), `KernelConfig::default()` (work budget 1,024, event buffer
4,096). Three complete independent release runs; equal deterministic truth hash
across all three (`4908686358519612288`). Raw data:
`reports/data/chron-032-chaos.json` and `reports/data/chron-032-memory.jsonl`.

| Metric | Value |
|---|---|
| Wall (min / median / max) | 1627.4 / 1677.7 / 1685.9 s |
| Sim-seconds per wall-second (median) | 187,969.8 |
| Committed outcome events total | 6,717,900 |
| Events per wall-second | ~4,004 |
| Action transitions total | 202,290,027 |
| Decision rounds total | 7,934,674 |
| Peak live scheduler entries | 200 (= 2 × population bound) |
| Peak scheduler heap nodes (live + stale) | 463 |
| Population | 100 preserved |
| Aggregate completions | 5,225,100 Work / 746,400 Eat / 746,400 Sleep |
| Standalone Move completions | 0 (persons reach sites via activity movement phases) |
| Peak RSS delta | 7,487,488 bytes cold (single cold sample, `kernel-10-year`) |

Eat/Sleep/Work + movement overlap: 100/100 persons completed all four required
observations. Idle observed = 0 (Work weight 2,300 vs Idle −50 with a fully
reachable Work/Meal/Rest fixture); the Idle instrument is separately unit-tested
(empty-site fixture). Death statistics and Event Store durability are
`NotApplicable` (in-memory Phase 1).

## Review correction 2026-08-31

Current030–032 evidence is [the review closeout](reports/CHRON-030_032_REVIEW_CLOSEOUT.md),
which supersedes the old delta-clock60/60/60FPS claim and distinguishes the old
colocated RSS fixture from the corrected chaos fixture. Historical raw files and
numbers above remain source-specific, not interchangeable current measurements.
Corrected normal UI:300 consecutive measured monotonic intervals, mean59.975FPS,
p95frame16.997ms; no constant60 claim. Corrected100-person ten-year run:1565.935s,
allrequiredbehaviors100/100, native RSSpeak6619136B, proven coldincrement5079040B.
RSSn=1 is an observation with no distribution/variance claim (owner preference,
ADR0028). No3/5/7GB or60FPS requirement has changed.

CHRON033 uses release code, one identical seed42 reachable fixture,86400seconds,
2warmups/10timings at100/1K/3K/5K/10K, one coldRSS per scale, and matching
unpaced direct/worker/windowed work. Missing/failing higher scales remain visible.
Native RSS, serialized bytes and rendered GPU-memory estimates are distinct.
NLG/history-query/fullrelationship metrics remain NotApplicable inPhase1.
