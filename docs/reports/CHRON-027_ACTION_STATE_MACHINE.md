# CHRON-027 — Action Execution State Machine Report

> Historical baseline. Current time/rejection contract is ADR-0024/0025;
> final local verification/measurement: [repair V2 report](P1_KERNEL_REPAIR_V2.md).

- Date: 2026-08-30
- Task: [CHRON-027](../tasks/CHRON-027.md); contract:
  [ADR-0021](../adr/ADR-0021-phase-1-action-execution-contract.md)
  (Accepted with the P1-REMAINING / 2026-08-30-r1 execution approval)
- Machine: Apple M5, 10 cores, 16 GiB, macOS 26.6.2; Rust 1.98.0 stable
  (workspace MSRV 1.95), `--release --locked`

## Change Summary

`palimpsest-sim-core` gains the authoritative Phase 1 action executor
(`src/actions.rs`, new public API, no new crate):

- `ActionRuntime` owns at most one execution record per person over a
  due-time/FIFO `Scheduler` (ADR-0004), with `start` / `advance` / `cancel` /
  `current` / `current_action` / `next_due` / `drain_events` / `stats` /
  `metrics`. `ActionState` covers `Idle`, `Moving { action }`, `Eating`,
  `Sleeping`, `Working`.
- Movement costs 1 simulated second per 4-directional cell; paths include the
  start cell, which is never re-walked. Eat/Sleep/Work carry a movement phase
  before the activity phase; standalone Move completes on arrival. Every
  action occupies at least one second, so same-instant completion loops are
  unrepresentable. Zero-distance arrival still ticks once.
- Durations (ActionConfig defaults, D1): Eat 600s, Sleep 28,800s, Work
  1,800s, Idle wait 60s, retry delay 1s, critical recheck delay 60s.
- Completion materializes needs growth to the completion instant first, then
  applies `Needs::eat(100_000)` / `rest(100_000)`; Work increments the bounded
  site `WorkCounter` only. Interrupted/blocked/failed actions get no reward.
- Start revalidates against simulation truth (site kind, `find_path`
  reachability) per the ADR-0019 executor boundary. Overlap returns
  `AlreadyExecuting` unchanged; unknown persons return `UnknownPerson`.
- Blocked/failed execution cancels the record's live continuation, commits one
  atomic transition to `Idle`, and schedules the retry decision at
  `now + retry_delay` (never same-instant). Each person holds at most two live
  scheduler tokens (continuation/retry + critical check); popped work whose
  token does not match the record is discarded, so stale/double delivery
  cannot execute twice.
- Per-person critical-need boundary checks are scheduled at the exact
  `CRITICAL_PRESSURE` crossing (integer ceiling division on committed raw
  needs); a person already critical is rechecked with a positive 60s delay.
  The executor never selects: it surfaces `DecisionRequest`s, and the
  reference driver (`decide_and_start` / `resolve_decision` / `run_until`)
  re-selects on the live context, interrupting only when a different
  `(kind, target)` wins (ADR-0014/0018).
- High-level outcomes (`action.completed/blocked/failed/interrupted/
  cancelled`) are validated schema-1 `EventRecord`s in a bounded 4,096-entry
  buffer with a visible rotation counter; Idle completions are counted, not
  emitted. `ActionStats` separates movement-phase completions from top-level
  completions.
- `sim-core` gains a normal `serde_json` edge (already locked workspace
  version) to fill event metadata; `Cargo.lock` is unchanged.

## Defect Found and Fixed During Development

The first `advance(now)` implementation committed every drained due item at
the *target* instant instead of its *own* due instant, so one long advance
diverged from per-instant stepping (caught by a unit test exercising
cancel → restart → long advance). `advance` now commits each item at its
recorded due instant, and a segmentation-equivalence test
(`one_long_advance_equals_per_instant_stepping`) locks the property. This
matters directly for CHRON-028's budget-split advance contract
(P1-REMAINING D2).

## Files Modified

- `crates/sim-core/src/actions.rs` (new)
- `crates/sim-core/src/lib.rs` (exports)
- `crates/sim-core/Cargo.toml` (`serde_json` normal edge; comment)
- `crates/sim-core/tests/action_closed_loop.rs` (new)
- `crates/sim-core/examples/action_execution_bench.rs` (new, timing + memory
  adapter)
- `tools/bench-memory/src/main.rs` (dispatch + case list)
- `tools/bench-memory/tests/cli.rs` (22 → 24 planned cases)
- `docs/adr/ADR-0021-phase-1-action-execution-contract.md` (new)
- `docs/reports/data/chron-027-action-bench.jsonl`,
  `docs/reports/data/chron-027-action-memory.jsonl` (raw artifacts)
- this report

## Tests

Covered scenarios (35 sim-core lib tests + 5 integration tests, all green):

- Every legal transition: Idle→Moving→Eating/Sleeping/Working→Idle,
  Idle-wait lifecycle, Move arrival completion, zero-distance 1s occupancy.
- Overlap: second start returns `AlreadyExecuting`, state unchanged (including
  during an Idle wait).
- Unknown person, repeated cancel (`InvalidTransition`), blocked/unreachable
  starts leave all state unchanged.
- Arrival-time site recheck → blocked recovery to Idle, `action.blocked`
  event, retry decision exactly at `now + retry_delay`; work-counter failure →
  failed recovery.
- Interrupt: single atomic transition, needs materialized without reward,
  continuation token cancelled (only the critical check remains live), no
  post-interrupt execution, `action.interrupted` event.
- Old/stale tokens cannot execute after cancel + restart (steps, completions,
  and event stream exactly match the surviving action).
- Critical boundary: exact crossing instant, materialization at the boundary,
  60s positive recheck pacing while critical; driver interrupts only when a
  different `(kind, target)` wins, and never when the current action is
  re-elected.
- Bounded event buffer: 4,096 cap, rotation counter exact, every drained
  event passes `validate()`, monotonic timestamps.
- Determinism: two identical driver runs produce byte-identical transition
  logs, event streams, and stats; segmentation equivalence (above).
- Metrics: at most two live scheduler tokens per person.
- Closed loop (ADR-0018 mandatory): seed 25,025 reference fixture, 172,800
  seconds, driven by real `candidate_actions`/`select_action` — positive Work,
  Eat, and Sleep completions; hunger is exactly 0 after every Eat completion
  and fatigue exactly 0 after every Sleep completion; the person returns to
  Work after needs recover; two full runs are identical (completions, events,
  stats, final needs/location).
- Unreachable world (one-cell path cap): the loop degrades to Idle waits,
  16 completions in 1,000s, no fabricated movement, needs commit at
  boundaries (hunger 960 / fatigue 1,920 at the last boundary).
- Driver-level interrupt: a critical fatigue boundary supersedes executing
  Work with Sleep through a real selection.

Commands run (all at the final source state):

```sh
cargo test -p palimpsest-sim-core                       # 34 lib + 5 integration + example adapter tests
cargo test --locked --workspace --all-targets           # full workspace, all green
cargo test --locked --workspace --doc                   # 2 doctests (person boundary)
cargo clippy --locked --workspace --all-targets --all-features   # 0 warnings
cargo fmt --all -- --check                              # clean
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
./tools/ci-rust.sh                                      # exit 0: spec hash, fmt, clippy -D warnings,
                                                        # workspace tests, MSRV 1.95, 7 release smokes
cargo test -p palimpsest-bench-memory --test cli        # 7/7 incl. 24-case registry
git diff --check
```

## Benchmark

Timing (release, 2 warm-ups, 10 samples, fresh fixture per sample; fixture
setup excluded from the timed interval; correctness assertions enabled):

```sh
cargo run --release --locked -p palimpsest-sim-core --example action_execution_bench -- --persons 100 --seconds 172800 --samples 10
cargo run --release --locked -p palimpsest-sim-core --example action_execution_bench -- --persons 1000 --seconds 172800 --samples 10
```

| Persons | Sim seconds | Median wall | min–max | Transitions | Completions (E/S/W) | Checksum |
|---:|---:|---:|---|---:|---|---:|
| 100 | 172,800 | 98.14 ms | 73.01–102.30 ms | 27,931 | 400 / 400 / 3,000 | 14135520335129562204 |
| 1,000 | 172,800 | 778.40 ms | 716.16–865.95 ms | 280,354 | 4,000 / 4,000 / 30,000 | 745488662195329176 |

That is ≈1.76M simulated seconds per wall second at 100 persons and ≈222K at
1,000 on this fixture. A repeat of the 100-person command recorded a 84.21 ms
median; the ~15% spread across the two invocations is ordinary short-run
noise, and the checksums are identical. Raw output:
`docs/reports/data/chron-027-action-bench.jsonl`.

Peak incremental RSS (REM-008A tool, ADR-0020; 3 fresh cold processes per
case; workload = one 86,400-second closed-loop day; kernel-proved
`baseline_at_lifetime_peak` on every sample):

```sh
target/release/palimpsest-bench-memory --run action-100 3
target/release/palimpsest-bench-memory --run action-1000 3
```

| Case | Cold peak increment min/median/max | Operation interval |
|---|---|---|
| action-100 | 3,424,256 B (×3 identical) | 2,899,968 / 2,916,352 / 2,932,736 B |
| action-1000 | 7,634,944 / 7,651,328 / 7,684,096 B | 6,799,368–6,832,128 B |

Cold intervals include fixture construction (world generation, spawn, first
touch); the operation interval isolates the closed-loop run itself. Raw
output: `docs/reports/data/chron-027-action-memory.jsonl`. Golden checksums
(4716271126859177484 / 9948480634061406840) are asserted by the adapter tests.
Scheduler queue health: at most 2 live entries per person (continuation +
critical check), confirmed by the metrics test; no unbounded growth.

## Known Limitations

- No NPC collision/occupancy, no dynamic re-planning, no economy — by design
  (D1, ADR-0021).
- The reference driver (`run_until`) is CHRON-027 scope; CHRON-028's kernel
  supersedes it as the orchestration owner.
- Outcome events are a bounded in-memory diagnostic sink with rotation, not
  durable retention; decision traces remain per-decision runtime diagnostics.
- Needs values commit at transition boundaries; a read between boundaries sees
  the last committed values (projection for display is CHRON-028/029 scope).
- The bench fixture clusters three sites near the spawn clearing; 1,000
  persons stride across the connected walkable region. `move_completions` is
  0 in this fixture because standalone Move never outbids Work/Eat/Sleep under
  the ADR-0018 table — movement happens inside the other actions
  (`movement_completions` > 0 is asserted).
- RSS is kernel-granular macOS accounting, not per-object heap size.

## Blockers

None.

## Next Ready Task

CHRON-028 (Scheduler / Kernel Orchestration) per the approved P1-REMAINING
DAG: 027 is its direct dependency and is now verified.
