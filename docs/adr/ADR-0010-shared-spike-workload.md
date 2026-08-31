# ADR-0010: Shared Phase 0 Mode-Comparison Workload

- Status: Retired by CHRON-035 (2026-08-31); historical decision retained
- Date: 2026-08-29

## Context

The final Phase 0 report must compare Headless and Rendered simulation speed.
Comparing unrelated implementations would not support an architectural
conclusion. The existing finite workload lived in the headless application,
which would force the Godot bridge to depend outward on an app adapter.

## Decision

Move the deterministic finite workload implementation into `sim-core` under
explicitly spike-scoped names. The headless runner re-exports it to preserve its
current API, while the Godot bridge invokes the same function through a
benchmark-only presentation method. Both modes measure only the shared Rust
workload using release builds and validate identical completion invariants.

This interface is measurement infrastructure, not a permanent gameplay API. It
must be reviewed or removed before Phase 1 turns `sim-core` into a real world
kernel.

## Consequences

The comparison uses identical code and data flow, and dependency direction stays
inward toward Simulation Core. `sim-core` temporarily contains a dummy benchmark
harness and gains a serde dependency for machine-readable metrics. Rendered-mode
measurement blocks the Godot main thread during each sample and therefore does
not model a future asynchronous simulation architecture.

## Alternatives Considered

- Bridge depends on `apps/headless-runner`: rejected; adapters must not depend on
  other outer adapters.
- Duplicate the workload in GDScript or the bridge: rejected; results would not
  be comparable.
- Compare uncapped headless execution with 60 Hz frame pacing: rejected; that
  answers scheduling-policy throughput rather than Rust kernel overhead.

## Retirement — 2026-08-31

After CHRON034 candidate8dc1595 passed both hosted checks, CHRON035 removed the
Core spike module/exports, headless mode-benchmark binary/alias and Godot spike
method. Default headless execution now drives the real seed42 WorldKernel;
Godot retains its worker/snapshot presentation path. The preserved Phase0 rationale
above is historical and must not be reused as a current performance claim.

[CHRON033](../reports/CHRON-033_SCALE_BENCHMARKS.md) supplies representative scales
and equal-work direct/worker/windowed results. [CHRON035](../reports/CHRON-035_SPIKE_RETIREMENT.md)
records removal and equivalent test coverage. Historical reports/raw artifacts
and3/5/7GB budgets are preserved. The old synchronous dummy comparison is not an
argument for blocking the Godot thread or for retaining a dummy in production.
