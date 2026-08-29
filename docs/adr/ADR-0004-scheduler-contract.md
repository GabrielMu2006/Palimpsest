# ADR-0004: Deterministic Scheduler Contract

- Status: Accepted for Architecture Spike
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for cross-system scheduling changes

## Context

Simulation systems need different cadences, and 10,000 intelligent entities cannot all run full updates every second. Scheduling must be deterministic, observable, and independent of rendering or ECS iteration.

## Decision

- Use a generic headless `Scheduler<T>` backed by a due-time priority queue.
- Order first by ascending `SimInstant`, then by ascending insertion order.
- Rescheduling retains the token but assigns a new insertion order.
- `ScheduleToken` is runtime-local, non-persistent, and distinct from `EntityId`.
- Cancellation and rescheduling invalidate old heap nodes lazily; explicit and threshold-based compaction bounds stale-node growth.
- The Scheduler returns payloads to its caller and never invokes callbacks internally. Follow-up scheduling therefore occurs explicitly after a pop.
- Developer metrics expose live entries, heap nodes, and stale nodes.
- Persisted Scheduler representation is deferred to the Snapshot task and must not serialize runtime heap internals blindly.

## Consequences

- Equal inputs produce stable execution order independent of hash-map iteration.
- Systems can process only due work instead of scanning every entity.
- Cancellation is constant-time on the live-entry map but may leave a temporary stale heap node.
- Token/order exhaustion is explicit and never wraps.
- The single-threaded queue is the Phase 0 baseline; concurrency requires measurement and a superseding ADR.

## Alternatives Considered

- Scan every entity each tick: rejected by the Simulation LOD and performance requirements.
- Execute boxed callbacks inside the queue: rejected because it obscures ownership, reentrancy, serialization, and system boundaries.
- Timing wheel: deferred until workload measurements show the heap is insufficient.
- Persist raw heap state: rejected because stale runtime nodes and internal ordering are not a stable snapshot schema.

