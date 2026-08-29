# ADR-0010: Shared Phase 0 Mode-Comparison Workload

- Status: Accepted for Architecture Spike
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
