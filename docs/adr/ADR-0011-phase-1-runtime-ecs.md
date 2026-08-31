# ADR-0011: Phase 1 Provisional Runtime ECS

- Status: Accepted by product-owner Phase 0 decision (2026-08-29, decision 2)
- Date: 2026-08-29
- Decision owners: Product owner confirmation required to change the runtime ECS or its identity mapping

## Context

Phase 0 measured `bevy_ecs` 0.19.1 against dummy two-component entities only
(CHRON-008). The product owner accepted it provisionally and explicitly did not
make it permanent. Phase 1 introduces representative Person, Location, Needs,
Movement, and schedule-backed behavior, so the runtime-ECS choice must be
re-examined before it becomes a durable commitment.

## Decision

Use `bevy_ecs` 0.19.1 as the Phase 1 provisional runtime ECS. It is a
replaceable implementation detail held behind stable domain identity, not a
final MVP commitment.

- Persistent identity remains the domain `EntityId` (see ADR-0002). Runtime
  `bevy_ecs::Entity` values are never persisted, serialized, or exposed through
  the storage or Godot boundary.
- Maintain a separate, non-persistent runtime mapping
  `EntityId -> bevy_ecs::Entity`, constructed when a world is rebuilt and
  discarded on teardown.
- `EntityId` is stored as a component so it survives iteration and is the only
  entity reference used by events, relations, snapshots, and client view models.
- Re-evaluate the choice after representative component benchmarks (Person,
  Location, Needs, Movement, plus a representative `bevy_ecs` System) and after rendered/headless
  and 100-NPC/10-year validation. Only a superseding ADR makes it permanent or
  replaces it.

## Public Contract

- `palimpsest-sim-entity::EntityId` remains the canonical, persisted identity.
- A runtime handle type (the `bevy_ecs::Entity` value) is a distinct, ephemeral
  concept that exists only within one world lifetime.
- The runtime-mapping layer is internal to the simulation kernel and is not part
  of the public storage, event, snapshot, or Godot-bridge contract.
- No public API accepts or returns a runtime ECS handle; all cross-boundary
  references use `EntityId`.

## Consequences

- Persisted schemas remain ECS-agnostic and stable across runtime changes.
- Rendering, history, and storage never encounter a `bevy_ecs::Entity`.
- The Rust 1.95 MSRV remains an explicit dependency cost (see ADR-0005).
- The dummy-entity throughput and memory figures are not full-NPC results and
  must not be presented as such; the next measurement must use real components.
- Behavior must be validated before the choice is treated as durable.

## Rejected / Deferred Alternatives

- Persist Bevy `Entity` values: rejected as identity corruption across runtime
  rebuilds.
- Build a custom ECS now: rejected without evidence that `bevy_ecs` fails real
  Phase 1 components.
- Switch runtime ECS during Phase 1: deferred until the real-component benchmark
  is measured; switching without evidence is speculative churn.

## Supersedes / Extends

Extends ADR-0005 (bevy_ecs spike) with a provisional Phase 1 commitment and
remains consistent with ADR-0002 (stable persistent identity). It supersedes the
"final decision pending" framing of ADR-0005 only for Phase 1 scope; it does not
claim finality.
