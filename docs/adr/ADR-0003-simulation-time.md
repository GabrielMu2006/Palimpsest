# ADR-0003: Simulation Time Representation

- Status: Accepted for Architecture Spike
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for changes to persisted time

## Context

The simulation must run faster or slower than real time, headlessly, and with system-specific update cadence. Wall-clock time and Godot frames therefore cannot define simulation truth.

## Decision

- `SimInstant` stores signed `i64` integer seconds from a simulation epoch.
- `SimDuration` stores non-negative integer seconds within `i64::MAX`.
- Both serialize as their numeric second representation.
- `SimClock` owns the current instant and permits only monotonic advancement.
- Checked arithmetic reports overflow instead of wrapping or saturating silently.
- The epoch-to-calendar interpretation remains outside the time primitive.
- Scheduler cadence is a consumer of these primitives and is decided separately.

## Consequences

- Headless and rendered modes share exactly the same simulation timeline.
- Systems can schedule work at different granularities without ticking every entity every second.
- Negative instants remain representable for pre-epoch setup or future calendar requirements, while elapsed durations remain non-negative.
- One-second resolution cannot directly represent sub-second combat; a superseding ADR would be required if measured product requirements demand finer truth resolution.

## Alternatives Considered

- Floating-point seconds: rejected because long histories and repeated arithmetic introduce drift and ordering ambiguity.
- Unsigned instants: rejected because signed epochs retain flexibility for pre-epoch setup without changing serialized width.
- Wall-clock timestamps: rejected because simulation speed, pause, replay, and headless execution must be independent of real time.
- A full calendar type in the core primitive: deferred because calendar semantics are product content, not required by the Architecture Spike.

