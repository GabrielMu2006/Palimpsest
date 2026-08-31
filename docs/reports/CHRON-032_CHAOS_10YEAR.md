# CHRON-032 — Headless 10-Year Chaos Runner (Phase 1 No-Crash Gate)

> Review update 2026-08-31: Original timings remain historical evidence, but movement counted activity completions and the original RSS used a different colocated fixture. The corrected one-run ten-year chaos/native-RSS result passes; see CHRON-030_032_REVIEW_CLOSEOUT.md and ADR-0028 for exact semantics and single-sample limits.
- Status: **Implemented** 2026-08-31. Authority: `MASTER_SPEC.md` (unchanged),
  ADR-0027 (this task's contract), ADR-0013/0021/0022/0024/0025 (kernel + action
  semantics). This is the **Phase 1 10-year no-crash correctness gate**, not a
  throughput or 200-year claim (CHRON-033/036 hold budgets; the 200-year claim
  is explicitly out of scope).
- Machine: M5 16 GB, macОS (local). Release build `--locked`.
- Raw evidence: `docs/reports/data/chron-032-chaos.json` (timing) and
  `docs/reports/data/chron-032-memory.jsonl` (peak RSS).

## Outcome

A fixed-seed (42) 100-person world ran headlessly through the CHRON-028 kernel
for **10 simulated years (`315_360_000` s, 365-day years)** and completed
**without a panic, NaN/infinite value, deadlock, dangling reference, or unbounded
scheduler queue**. All 100 persons persisted (Phase 1 has no removal), every one
completed Eat, Sleep, Work, and a real movement phase, and the run was byte
deterministic across three independent executions. The Phase 1 "continuous 10
years without crash" gate is **met**.

## Configuration

`ChaosConfig { seed: 42, person_count: 100, years: 10, sim_seconds_per_year: 31_536_000 }`,
`KernelConfig::default()` (work budget 1,024 due-instant rounds, event buffer
4,096). Spawn cells were resolved deterministically to a connected walkable
component containing a Meal, a Rest, and a Work site, so every person has a real
path to each — no teleport, no manufactured selection, no weight change
(ADR-0018 defaults retained).

## Measured results

| Metric | Value |
|---|---|
| Wall (min / median / max) | 1627.4 / 1677.7 / 1685.9 s |
| Sim-seconds per wall-second (median) | 187,969.8 |
| Committed outcome events | 6,717,900 |
| Events per wall-second (median) | ~4,004 |
| Action transitions | 202,290,027 |
| Work completions | 5,225,100 |
| Eat completions | 746,400 |
| Sleep completions | 746,400 |
| Standalone Move completions | 0 (persons reach sites via activity movement phases) |
| Peak live scheduler entries | 200 (= 2 × population bound) |
| Peak scheduler heap nodes | 463 |
| Peak RSS cold delta | 7,487,488 bytes (~7.14 MiB), single cold sample |

All three runs produced the identical truth hash `4908686358519612288`, identical
per-run event total/digest, and identical final instant — determinism holds.

## Coverage of detectors / violation tests

Every detector is a deterministic predicate in `crates/sim-core/src/chaos.rs`;
each returns a typed `ChaosError` and is proven by a unit test:

| Detector | Meaning | Proven by |
|---|---|---|
| `NonFinite` | needs/quantity outside `[0, 100_000]` | `needs_in_bounds_rejects_out_of_range` |
| `QueueGrowth` | live entries > 2×population or heap > 8×population | `queue_detector_fires_on_growth` |
| `DanglingReference` | event actor not a live population id | `dangling_reference_detector_fires` |
| `Invariant` | buffer `total = delivered + buffered + rotated`, population preserved | per-day check + `deterministic_two_day_run_is_stable_and_population_preserved` |
| `NonTerminating` / `Watchdog` | committed instant stalls across `MAX_STALLED` calls | bounded-progress guard in `run_chaos` |
| `ChaosError::Config` | zero population / non-positive horizon rejected | `zero_person_count_is_rejected` |
| Determinism | same seed ⇒ same truth hash; different seed ⇒ different world | `deterministic_two_day_run...`, `different_seed_produces_different_world` |
| Fixture reachability | seed-42 component has Meal + Rest + Work | `fixture_has_three_site_kinds_reachable` |
| Idle instrument | reports Idle when Idle is the only viable action | `idle_is_detected_when_no_site_is_reachable` |

## Commands actually run

```sh
cargo run --release --locked -p palimpsest-headless-runner --bin chaos_runner -- \
    --seed 42 --persons 100 --years 10 --runs 3 --out docs/reports/data/chron-032-chaos.json
./target/release/palimpsest-bench-memory --child kernel-10-year > /tmp/chron32-mem-one.json
# then the memory sample was normalized into docs/reports/data/chron-032-memory.jsonl
```

Workspace gate (all pass locally):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --release --locked --workspace --all-targets --all-features   # all green
cargo test --locked --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo +1.95.0 check --workspace --all-targets --all-features            # MSRV 1.95
cargo metadata --locked --no-deps --format-version 1
cargo tree --locked --workspace --edges normal                          # dependency review, no new deps
cargo test --release --locked -p palimpsest-bench-memory --test cli     # 8 (29-case list)
sh tools/ci-godot.sh                                                    # exit 0
git diff --check ; shasum -a 256 MASTER_SPEC.md  # unchanged hash
```

## Population / completeness gate

`persons_completed_all_kinds == 100` (Eat + Sleep + Work + movement phase for
every person). Idle is reported, not gated, and was genuinely unobserved
(`idle_observed_persons = 0`) under the ADR-0018 default weights (Work 2300 vs
Idle −50) with a fully reachable Work/Meal/Rest fixture. The Idle reporting path
is separately proven by `idle_is_detected_when_no_site_is_reachable` (empty-site
fixture → Idle is the only viable action). This is a Phase 1 product-semantics
finding, not a hidden failure: Idle here is the negative-weighted do-nothing
baseline, correctly never selected while the world is fully served.

## Known limitations

- **In-memory only**: no save/load. Database/Event-Store durability and database
  consistency are `NotApplicable` (Phase 1 chaos run is explicitly in-memory).
- **Death statistics** are `NotApplicable` (no Phase 1 ageing/birth/death).
- **No economy/ageing/family/relations** (Phase 2+); the 200-year claim and the
  Phase 1 total budget/scale gates remain with CHRON-033/036.
- **Standalone `Move` is never selected** in this fixture; movement is proven via
  each activity's real reach phase (`movement_phases = 6,717,900`).
- **Idle = 0** (see above); the instrument is correct, the fixture/weights never
  select it.
- **Peak RSS is a single cold sample** (user directed one cold run; the tool's
  3-cold median was not repeated). Reported as a cold peak increment; the
  CHRON-036 budget decision should re-measure with the full 3-cold protocol.
- **Hosted required checks have not run** on this uncommitted tree; local gates
  (see below) are verified, but hosted `rust-quality-and-smoke-benchmarks` /
  `godot-macos-integration` have not.
- Third-run wall (1685.9 s) vs first (1627.4 s) differ by ~3.5% — machine load,
  not simulation non-determinism (the truth state is identical).

## Blockers

None. CHRON-032 is complete and locally green.
