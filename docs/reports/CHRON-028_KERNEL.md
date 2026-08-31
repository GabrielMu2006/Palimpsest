# CHRON-028 — Scheduler / Kernel Orchestration

> Historical implementation/pilot report. Current contract: ADR-0024/0025;
> final local verification and replacement measurement: [repair V2 report](P1_KERNEL_REPAIR_V2.md).
> The old "None" blocker statement below is not current verification.

## Change Summary

Added the Phase 1 headless world kernel, `WorldKernel`, as the single owner of
time advancement and ordering over the person/terrain/action world.

- New module `crates/sim-core/src/kernel.rs` (ADR-0022), exporting from
  `crates/sim-core/src/lib.rs`:
  - `WorldKernel` — owns `SimClock`, `WorldMap`, `ActivitySites`,
    `PersonRuntime`, `EntityIdAllocator`, `ActionRuntime`, the decision
    weights/perturbation, the latest per-person `DecisionTrace`, and a bounded
    kernel event buffer.
  - `KernelConfig` (`action`, `weights`, `perturbation`, `work_budget`,
    `event_buffer_capacity`), `DEFAULT_WORK_BUDGET` (1,024),
    `DEFAULT_EVENT_BUFFER_CAPACITY` (4,096).
  - `advance_to(target, work_budget)` + `advance(target)` (configured budget),
    returning `KernelAdvance` (`committed_to`, `rounds`, `reached_target`,
    `transitions`, `decisions`, `events`).
  - `new`/`from_world`, `spawn_person`, `start_world`, `now`, `next_due`,
    `person_count`, `person`, `persons`, `latest_trace`, `drain_events`,
    `metrics`.
  - `KernelPersonView`, `KernelMetrics`, `KernelError` (wrapping clock,
    identity, person, action, decision, and event errors).
- `crates/sim-core/src/actions.rs`: added `serde` derives to `ActionState`
  (required so the render DTO can project it; no behavior change).
- New integration test `crates/sim-core/tests/kernel.rs` (9 tests).
- New benchmark example `crates/sim-core/examples/kernel_bench.rs`.
- ADR-0022 (kernel contract) recorded and accepted with the Task.

## Semantics Implemented (per ADR-0022, P1-REMAINING D2)

- Kernel jumps between due instants (ADR-0004); it never scans every person
  every simulated second. Each round runs `actions.advance(d)` (all work due at
  `d`, due-time/FIFO), resolves every surfaced `DecisionRequest`, and advances
  the clock to `d`.
- `advance_to` is bounded by `work_budget`; exhausting the budget yields
  `reached_target == false` with the actual `committed_to` so the caller may
  continue. Splitting a target across several calls is exactly equivalent to a
  single long advance (segment-equivalence test).
- Clock regression (`target < now`) returns `KernelError::ClockRegression`
  with no mutation; equal-target advances are clean no-ops.
- `start_world` is the only seed step (runs `decide_and_start` per person); the
  kernel never auto-kicks idle persons during advance.
- Per-person latest `DecisionTrace` retained; high-level outcome events are
  validated, counted, and appended to a bounded buffer with a visible rotation
  counter.
- Deterministic: identical seed/config/spawn/advance sequence yields
  byte-identical event and decision summaries and identical visible state.

## Commands Actually Run

```sh
cargo build -p palimpsest-sim-core
cargo test --workspace --all-targets --all-features        # all green, incl. 9 kernel tests
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo +1.95.0 check --workspace --all-targets --all-features           # MSRV 1.95 clean
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps              # clean
cargo fmt --all -- --check
cargo run --release --locked -p palimpsest-sim-core --example kernel_bench -- --persons 100 --seconds 86400 --samples 1
```

## Benchmark Result

Release, Apple M5-class reference machine, one sample (pilot; the authoritative
two-warm-up/ten-sample M5 run is recorded in `docs/PERFORMANCE.md`):

```json
{"persons":100,"seconds":86400,"wall_seconds":0.435,"sim_per_wall":198731.9,
 "rounds":635,"transitions":65900,"decisions":1800,"events":1800}
```

Raw pilot output: `docs/reports/data/chron-028-kernel.jsonl`.

## Test Coverage

- `clock_regression_is_rejected_without_mutation`
- `equal_target_advance_is_a_noop_after_committing`
- `budget_exhaustion_yields_and_is_resumable_to_the_same_truth`
- `repeated_runs_are_deterministic`
- `all_persons_are_addressable_by_stable_entity_id`
- `decisions_and_events_are_accounted_and_bounded`
- `cadence_jumps_due_instants_and_never_accesses_every_second`
- `empty_world_advances_to_the_target_directly`
- `after_reaching_a_target_no_due_work_remains_at_or_before_it`

## Known Limitations

- Single-threaded and in-process; no persistence (in-memory validation only).
- `committed_to` is the last fully committed instant; on a mid-advance error
  work committed before the failure is not rolled back (documented).
- The 100-NPC/10-year gate belongs to CHRON-032; this Task records the
  per-day kernel throughput, not the multi-year claim.
- Formal two-warm-up/ten-sample and peak RSS evidence is complete in repair V2
  R2-05; see the linked final report, not this historical pilot.

## Blockers

None. Implementation is green and deterministic.
