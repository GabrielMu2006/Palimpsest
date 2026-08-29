# ADR-0005: Continue the bevy_ecs Architecture Spike

- Status: Accepted for continued Phase 0 validation
- Date: 2026-08-29

## Context
The Master Spec recommends standalone `bevy_ecs` as a hypothesis, while requiring stable persistent identity and measured 10K scale.

## Decision
Continue using `bevy_ecs` 0.19.1 for Phase 0 runtime-ECS experiments. Keep `EntityId` in a domain component and maintain a separate `EntityId -> bevy_ecs::Entity` runtime map. Never serialize the runtime handle. Raise workspace MSRV from 1.85 to 1.95 because bevy_ecs 0.19.1 requires Rust 1.95; the installed reference toolchain is Rust 1.98.

This is not a final MVP commitment. Re-evaluate after representative components, Scheduler integration, snapshots, and rendered/headless comparison exist.

## Evidence
On the M5 16GB reference machine, 10K entities with two small components and a stable runtime map used an estimated 2.30 MiB process-RSS delta over the same empty benchmark process. Five 1,000-step samples produced a median 1.270 billion simple component updates/s. Exact method and limitations are in `CHRON-008_10K_DUMMY_BENCHMARK.md`.

## Consequences
- Current throughput and memory do not reject the hypothesis.
- The Rust 1.95 MSRV is an explicit dependency cost.
- Bevy remains outside persistent domain schemas and the Godot boundary.
- Dummy component throughput must not be presented as full NPC simulation throughput.

## Alternatives Considered
- Persist Bevy Entity values: rejected as identity corruption.
- Pin an older Bevy solely to retain an unrequired Rust 1.85 MSRV: rejected because no product MSRV exists and the reference toolchain is newer.
- Build a custom ECS now: rejected without evidence that Bevy fails the spike.

