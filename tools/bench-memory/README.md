# Benchmark memory tool (REM-008A)

This outward-only binary measures the existing CHRON-019..029 fixtures. It is
not part of the game runtime. Production crates never depend on it.

```sh
cargo build --release --locked -p palimpsest-bench-memory
target/release/palimpsest-bench-memory --list
target/release/palimpsest-bench-memory --run all 3
target/release/palimpsest-bench-memory --run utility-1000-25 3
cargo test -p palimpsest-bench-memory --release --test cli
```

Only macOS has a native measurement implementation. Other platforms can build
the tool and run portable contract/CLI tests, but measuring returns an explicit
unsupported error. `--run` accepts 1..100 samples. Each sample starts a fresh
copy of this binary, sequentially; no memory warmups, retry, best-of filtering,
polling thread, subprocess `ps`, privileged attachment or platform spoofing.
Do not run CPU-heavy tests concurrently with performance measurements.

The workspace Rust CI job checks portable contracts on Linux; the existing
macOS integration job also runs the native CLI/probe tests. Required-check
names, permissions and existing test steps are unchanged.

## Output and interpretation

One JSON line per case contains all raw samples and cold-peak min/upper-median/
max. Each sample records PID, index, checksum and two intervals:

- `cold`: just before fixture setup to the validated live operation result.
- `operation`: prepared fixture to the same validated live result.

Each interval records starting and ending `current_bytes` and
`lifetime_peak_bytes`. `peak_increment_bytes` is the kernel RSS peak during
the interval minus its starting **current** RSS, with proof
`baseline_at_lifetime_peak` or `new_lifetime_peak_in_interval` (ADR-0020).
Otherwise `proof=ambiguous_prior_peak` and the value is null; an ambiguous
**cold** interval fails the run rather than pretending to close the requirement.
Failed child exit, malformed result and changing checksums also fail the run.
Previously printed successful case lines are partial evidence, not a completed
batch when the command exits unsuccessfully.

Cold increments include fixture construction, first-touch code, allocator
metadata and callback overhead. They are not object heap sizes, warmed steady
state or standalone per-operation overhead. Neither RSS nor an increment is
Apple physical footprint/unified-memory usage. OS page granularity and memory
pressure apply. A zero prepared increment means no extra resident pages beyond
the retained baseline, not zero allocations or zero memory use.

| Case | Prepared before operation | Measured operation; result lifetime |
|---|---|---|
| grid | selector only | input Vec + LocalGrid + full scan; grid retained |
| worldgen-0/1/42 | seed/default config | one map + golden hash; map retained; no JSON serialization |
| person-100/1000 | count only | fresh ECS runtime, stable IDs, components and visible-state checks; runtime retained |
| needs-100/1000 | count/hour duration | Vec + 8,760 advances/person + MAX assertions; Vec retained |
| sites | seed map, 20 sites, work coordinates | 10,000 nearest queries + cloned-site 10,000 work updates; original fixtures retained, clone released |
| path-* | seed-42 map, BFS/query selection and expansion diagnostics | one selected A* and path/outcome assertions; result retained |
| candidates-100/1000 | seed-0 map, six sites, strided persons | enumerate and retain all 861/9,060 per-candidate traces; full input checks |
| utility-count-epsilon | seed-0 map, six sites, all-site-connected persons, ten candidates each, weights/spec | select and retain every full Selection + score/trace/checksum checks |
| action-100/1000 | seed-25025 map, three reference sites, strided persons in the connected walkable region | one 86,400-second decide/execute closed loop with completion assertions; fixture retained |
| kernel-100-year | seed42/default sites, 100 people colocated at first walkable cell; started at epoch | 31,536,000 simulated seconds, default 1,024-round advance budget, full truth/counters verified; kernel retained |
| render-control-100 | seed42/default sites, first 100 row-major walkable cells; advanced to600s | read-only full person views/checksum; kernel retained |
| render-snapshot-100 | identical render preparation | same read-only check plus schema2 build/serialize/validated roundtrip and truth comparison; kernel, DTO and bytes retained past second callback |
| worker-100-day | seed42/default sites, 100 people colocated at first walkable cell; worker thread created and paused with its initial snapshot | one 86,400-second AdvanceTo driven through the command queue until acked; worker, channels and final snapshot retained past second callback |

Kernel/read-only/snapshot/worker selectors are included in the 28-case `--list`.
The year workload runs only on explicit measurement, never as a unit-test body;
its adapter logic has a short two-person/day test. `--run all` now includes the
year workload and can take substantially longer. Render control is measured
inside the observation interval, not during preparation. Snapshot RSS includes
validation allocations as well as the retained DTO/bytes; subtracting two cold
peaks is not a pure allocation-cost measurement. Zero-person timing JSON uses
null for bytes/person. See ADR-0025 and P1_KERNEL_REPAIR_V2 for final evidence.

Path suffixes: trivial, short, medium, long, unreachable, node_budget,
path_budget. Epsilon is 0 or 25. Utility fixture setup intentionally runs its
original reachability tests, so cold cost is not selection-only cost. The
small operation can be hidden beneath a larger fixture-preparation lifetime
peak; this is why both intervals and the proof are stored.

The original eight example timing commands still use two warmups/ten samples
with unchanged assertions and no native callbacks. Their before/after `ps`
fields remain historical retained-endpoint series, not peak RSS. See
`docs/reports/REM-008A_MEMORY.md` for measured results and source identity.

## Native boundary and probes

`src/rss.rs::read` is the native measurement boundary (ADR-0020). It reads the
calling task's Mach current RSS/lifetime RSS peak using the already locked
libc ABI. All other code denies unsafe; domain workspace crates still forbid
it. The narrowly expected libc self-port deprecation is documented in source,
not a warning suppression on production code.

Test-only diagnostic CLI cases (excluded from `--list`/`all`) include
probe-noop, probe-retained, probe-transient, probe-contaminated, probe-fail.
The separate private `probe_pages` native boundary maps/touches/unmaps a fixed
64 MiB region for diagnostics only; it does not replace the workload allocator.
The retained/transient probes physically touch 64 MiB. Tests require a peak
after release substantially above the final endpoint and reject an earlier
peak as proof of a later small interval. `probe-fail` intentionally panics to
verify failure propagation; it must never produce a successful measurement.

## Phase1 closeout adapters (ADR0028/0029)

These are separate binaries; the existing29-case CLI/list contract is unchanged.

```sh
cargo build --release --locked -p palimpsest-bench-memory --bins
# Exact100-person seed42 chaos fixture,315360000seconds,one cold process:
target/release/chaos_memory
# Representative one-day fixture at one requested scale,one cold process:
target/release/micro_memory 1000
```

`chaos_memory` also retains3650 daily current-RSS observations; these are not
instantaneous peaks. `micro_memory` retains the final snapshot and its serialized
bytes through the native end read. Both report cold/prepared baselines and proof
status. The high-water interval ends before final adapter-output encoding.
Unknown increments stay null, not zero; n=1 has no variance/repeatability estimate.
The owner's reduced-repeat policy does not change any memory cap.
