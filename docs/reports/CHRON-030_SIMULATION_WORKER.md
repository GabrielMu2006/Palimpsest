# CHRON-030 — Simulation Worker / Command Bridge

> Review update 2026-08-31: Worker control interruption, shutdown ordering, and paired publications were repaired; see ADR-0028 and the review closeout. Historical benchmark artifacts below retain their original source identity.
> Implemented and locally verified 2026-08-31 by the main agent (Kimi Code CLI).
> The CHRON-030 handoff's `gpt-5.6-luna` subagent dispatch is unavailable in
> this runtime (no `collaboration.spawn_agent`); per the dispatch skill's own
> fallback rule the main agent implemented the task directly. This is a
> reported limitation, not a silent substitution.
> Contract: [ADR-0015 Phase 1 supplement](../adr/ADR-0015-simulation-worker-command-render-snapshot.md).

## Change Summary

Added the Phase 1 simulation worker: one dedicated `std` thread owns the
`WorldKernel`; callers submit bounded commands and read the latest published
immutable `RenderSnapshot`. Safe Rust, standard library only; no new
dependency, no IPC, no thread pool, no async runtime, no multi-threaded ECS.

- New module `crates/sim-core/src/worker.rs`, exported from
  `crates/sim-core/src/lib.rs`:
  - `SimulationWorker` — `new(kernel)`, `submit(command)`, `command_status`,
    `latest_snapshot`, `status`, `is_paused`, `speed`, `shutdown`; `Drop`
    requests shutdown and joins the worker thread.
  - `WorkerCommand` — closed set `Pause`/`Resume`/`SetSpeed`/`Step`/
    `AdvanceTo`/`Shutdown`; `SpeedMultiplier` — closed 1/5/20/100/1000/MAX
    set with `from_u32` validation.
  - `CommandSequence`, `CommandAck`, `CommandOutcome`, `CommandStatus`
    (`Unknown`/`Pending`/`Evicted`/`Completed`), `WorkerPhase`, `WorkerStatus`,
    `WorkerError` (`Full`, `Closed`, `InvalidSpeed`, `InvalidStep`,
    `NotPaused`, `ClockRegression`, `TickOverflow`, `KernelFaulted`,
    `KernelNotStarted`).
  - `COMMAND_QUEUE_CAPACITY` = 64, `ACK_LOG_CAPACITY` = 1,024,
    `MAX_STEP_STEPS` = 1,000.
- `crates/sim-core/src/kernel.rs`: the existing `#[cfg(test)]` fault-injection
  hook is now `pub(crate)` so worker unit tests can fault a kernel. No
  production behavior change.
- New integration tests `crates/sim-core/tests/worker.rs` (15 tests) and 7
  unit tests in `worker.rs`.
- New benchmark example `crates/sim-core/examples/worker_bench.rs` with a
  direct-kernel control, publication-latency and command-throughput
  measurement, and a short-window pacing diagnostic.
- `tools/bench-memory`: new `worker-100-day` case (28 cases total);
  README/CLI test updated.
- ADR-0015 gained the Phase 1 supplement fixing the thread/ack-retention/
  shutdown/error/pacing contract before implementation.

## Semantics Implemented (ADR-0015 supplement, P1-REMAINING D3)

- Commands are applied only between kernel calls, at a complete committed
  boundary; a command submitted mid-advance takes effect after the in-flight
  bounded call returns. Publication likewise happens only between kernel
  calls.
- `submit` returns `Full` at 64 queued commands and `Closed` after shutdown;
  a sequence is consumed only by a successfully enqueued command. Every
  enqueued command produces exactly one ack with the real committed boundary;
  rejections and shutdown preemptions are never reported as success.
- `Step(0)` is a side-effect-free no-op; `Step` > 1,000 or while running is
  `InvalidStep`; `AdvanceTo` while running is `NotPaused`; a regression target
  is `ClockRegression`; `now + steps` overflow is `TickOverflow`.
- Speed changes wall-clock pacing only (anchor + floor(elapsed × m)); MAX
  never waits on the wall clock and never skips simulation work.
- The exchange holds one latest slot (≤ 2 exchange-owned snapshots including
  the reader's frame); the publication sequence is monotonic; publication is
  forced on initial/pause/step/advance-to/shutdown and throttled to 10 Hz
  wall-clock while running. Publication precedes the acknowledgement, so an
  observed ack implies the boundary is visible.
- A kernel fault moves the worker to `Faulted`: cause exposed via `status()`,
  last complete publication retained, no new DTO built, advance commands
  rejected `KernelFaulted`, `Pause`/`SetSpeed`/`Shutdown` still accepted.
- Shutdown has an independent atomic stop path that works with a full queue;
  on close the worker sets `closing` under the sequence lock (so no command
  can slip in afterwards), rejects everything still queued as `Closed`,
  publishes the final boundary if it advanced, and marks the phase `Closed`.
- The worker starts paused and publishes an initial snapshot before `new`
  returns. `new` rejects a faulted kernel and a never-started non-empty
  `Setup` kernel.

## Commands Actually Run

```sh
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
./tools/ci-rust.sh                                   # exit 0 (fmt, deny-warnings clippy,
                                                     #  workspace tests, MSRV 1.95, 7 smokes)
cargo test --release --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo metadata --locked --no-deps --format-version 1
cargo tree --locked --workspace --edges normal       # sim-core deps unchanged
./tools/ci-godot.sh                                  # exit 0
cargo test --release --locked -p palimpsest-bench-memory --test cli
cargo test --locked -p palimpsest-sim-core --test worker   # 15/15, also 3 repeat runs
cargo run --release --locked -p palimpsest-sim-core --example worker_bench -- \
    --persons 100 --seconds 86400 --warmups 2 --samples 10 --json
cargo build --release --locked -p palimpsest-bench-memory
target/release/palimpsest-bench-memory --run worker-100-day 3
```

All passed. `ci-rust.sh` includes the MASTER_SPEC hash check (unchanged
`a6fa0654…`). No commit, push, PR, or remote change was made.

## Benchmark Result

Release build, Apple M5 / 16 GiB, 2026-08-31; two warm-ups + ten samples,
100 persons, seed 42, 86,400 simulated seconds, colocated first-walkable
fixture (identical to `kernel_bench`). Raw data:
`docs/reports/data/chron-030-worker-bench.json` (timing) and
`docs/reports/data/chron-030-worker-memory.jsonl` (3 cold processes).

| Metric | Median (min–max) |
|---|---|
| Direct-kernel control wall | 409.901 ms (408.876–442.509) |
| Worker submit→ack wall | 417.379 ms (415.385–440.850) |
| Worker overhead ratio | 1.016× |
| Submit→publication visible | ≤ ack time (publication precedes ack; median 417.379 ms from submit, dominated by the advance itself) |
| Command throughput (paused no-op steps) | 4,757,259 commands/s |
| Max observed queue depth | 64 (= capacity) |
| Worker/direct truth checksum | identical every sample: `6363214172540219169` |

Pacing diagnostic (short real wall-clock windows, not a reproducible trace):
1× advanced 0 sim-s in a 300 ms window (sub-second pacing, correct); 5×/20×/
100×/1000× advanced 1/5/30/309 sim-s in ~300 ms windows (order matches the
nominal factor); MAX advanced 141,389 sim-s in ~0.78 s (≈181,784
sim-s/wall-s vs ≈202,840 for the direct kernel year rate — the worker adds
publication and command overhead only).

Memory (`worker-100-day`, macOS kernel RSS high-water proof, ADR-0020):

| Interval | 3 cold samples (bytes) | Median |
|---|---|---|
| cold (fixture + worker + one-day advance) | 5,488,640 / 5,259,264 / 5,259,264 | 5,259,264 |
| operation (prepared worker → acked day) | 3,686,400 / 3,670,016 / 3,653,632 | 3,670,016 |

## Test Coverage

`crates/sim-core/tests/worker.rs` (15): initial paused + initial snapshot;
paused no-advance; exact step + stays paused; step-0 no-op without
publication; step validation (oversized, unpaused); advance-to reach/
regression/equal-target no-op; advance-to while running rejected; pacing
progress + MAX non-waiting (loose bounds); queue Full at capacity then drain;
shutdown command → Closed + later submit rejected + last snapshot retained;
independent stop path with a full queue (queued commands rejected `Closed`);
identical seed + command sequence → identical snapshot sequence; slow reader
keeps a complete older snapshot, publications never regress, every observed
publication validates; ack lifecycle `Unknown`/`Pending`/`Completed`/`Evicted`
past the 1,024 window; empty-`Setup` kernel advances through the worker.

`worker.rs` unit tests (7): exact pacing-target mapping for 1/5/20/100/1000,
saturation at the timeline end, `from_u32` closed-set validation, faulted
kernel rejects advance commands but accepts `Pause`/`SetSpeed`, stop-flag
preemption never reports the target reached, `Step(0)` is side-effect-free,
`new` rejects faulted and never-started kernels.

`worker_bench` example tests (6): worker matches the direct control on a
short horizon (checksum equality), throughput flood applies every command,
memory-adapter selector validation, plus protocol helpers.

## Definition of Done — Evidence Mapping

- One in-process worker owns the kernel and mutates only at whole committed
  boundaries: `worker.rs` (single thread, `Loop` applies between calls);
  tests `advance_to_reaches_the_target_and_validates`, determinism test.
- Bounded queue, `Full` on saturation, never unbounded blocking or silent
  drop: `bounded_queue_reports_full_then_drains`, `COMMAND_QUEUE_CAPACITY`.
- Latest-complete publication, never partial/backwards; latency measured, not
  promised: `slow_reader_…`, `initial_snapshot_…`, bench publication field.
- Deterministic pause/speed/step: unit pacing map tests + integration step/
  advance tests + determinism test.
- No IPC/process/multi-threaded ECS: std-only implementation; dependency tree
  unchanged (`cargo tree` output identical in shape; no manifest change).
- ADR-0015 conformance incl. ≤2 exchange-owned snapshots: ADR-0015 supplement;
  exchange holds one slot; slow-reader test holds two handles total.
- Worker overhead measured: 1.016× vs direct control (above).

## Known Limitations

- Single-threaded in-process worker by design; IPC, a separate process, and
  multi-threaded ECS remain deferred (ADR-0015).
- Wall-clock pacing is not a reproducible input trace; pacing numbers are
  short-window diagnostics, not throughput gates.
- The `commands/s` figure measures no-op-step round trips through a spin-poll;
  it is a queue/ack path benchmark, not gameplay throughput.
- Publication latency is reported as submit→visible for a long advance; the
  per-commit publication delay after an ack-adjacent boundary is bounded by
  the 100 ms throttle and the publish-before-ack rule, and is not separately
  instrumented at sub-millisecond resolution.
- Headless callers driving the kernel directly are unaffected; the Godot
  bridge does not yet consume the worker (CHRON-031).
- The Luna subagent dispatch required by the handoff was not available in
  this runtime; the main agent implemented and verified everything directly.
- Work remains uncommitted on `phase-1-planning` per the plan (publication
  happens in CHRON-034); hosted CI has not run on this tree.

## Source Identity (SHA-256, 2026-08-31)

| File | SHA-256 |
|---|---|
| crates/sim-core/src/worker.rs | `a22ee67a1848b36d50dca1de1172cf57d57ebb1575f45d1b0aac859f2cc31c47` |
| crates/sim-core/src/lib.rs | `a28f5366fabc272834e608474743f783fe1527f2bc7b38321d49b6c008bf5f1e` |
| crates/sim-core/src/kernel.rs | `18a0c5a119cdb0760a3ca43e538be7b49e486ec4b95d485dc3a6df9f97ffff50` |
| crates/sim-core/tests/worker.rs | `c18e1277591ad0333065b868873e49a52ff099aef9108b4e713d6cfb91cb2c88` |
| crates/sim-core/examples/worker_bench.rs | `8f623db0bfd9fbb7634274392a04a911eede2b3efcc1d443f5cd09f6d03c1791` |
| tools/bench-memory/src/main.rs | `0a5e898b1db041003dc8aeba358d57d27f055cb15052dd003858fb601b24ccbe` |
| tools/bench-memory/tests/cli.rs | `dbcb4861fee36d58b616e2f1ec13c1c0c3b5718fc7e0f7dfae161cd10e05c5fa` |
| docs/adr/ADR-0015-simulation-worker-command-render-snapshot.md | `22504f3930aefd4e9a9aa9b3c32c44c968a44ab0bbbd742e16738109718be4f8` |
| Cargo.lock | `fc9e78b4a732ca278bfcfdce4202adb8c0dc5d3aff42395a5ad2a855187813f0` (unchanged by this task; no dependency change) |
| MASTER_SPEC.md | `a6fa0654…` (unchanged, read-only) |

## Next Ready Task

CHRON-031 (Godot micro-world presentation) depends on 030 and is the next
task in the approved DAG; it is **not** authorized by this report and was not
started. CHRON-032 is also ready (depends on 028 only) but likewise not
started.
