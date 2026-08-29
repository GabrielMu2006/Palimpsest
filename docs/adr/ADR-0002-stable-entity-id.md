# ADR-0002: Stable Persistent Entity Identity

- Status: Accepted for Architecture Spike
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for changes to persistent identity

## Context

Events, relations, snapshots, databases, documents, and the Godot bridge need an identity that remains valid when runtime ECS worlds are rebuilt. An ECS entity handle may contain an arena index and generation that are meaningful only inside one runtime world.

## Decision

Define canonical persistent identity in the Godot-independent `palimpsest-sim-entity` crate:

- `EntityId` is an opaque `NonZeroU64` newtype and occupies one `u64`.
- Zero is invalid and reserved for external sentinel use.
- Serde represents `EntityId` as a plain unsigned integer, independent of Rust type layout or ECS choice.
- IDs are allocated monotonically from 1 and are never recycled within a world.
- Serializable allocator state records the next ID. Zero in that private allocator field means the `u64` ID space is exhausted.
- Loading existing entities must restore allocator state or advance it beyond the maximum restored ID before new allocation.
- Runtime ECS handles must be maintained in a separate, non-persistent runtime mapping.

The Architecture Spike does not yet choose an ECS. `bevy_ecs` integration and its runtime mapping remain a separate measured decision.

## Consequences

- Persistent references survive runtime world and client reconstruction.
- Numeric IDs are compact, sortable, database-friendly, and human-debuggable.
- A world has a finite maximum of `u64::MAX` allocated identities; exhaustion is explicit rather than wrapping.
- Mergeable distributed world shards are not supported by this allocator and would require a superseding ADR.
- Snapshot and database schemas must preserve allocator progress to prevent reuse.

## Alternatives Considered

- Persist ECS handles: rejected because they are runtime-local and would couple storage to one ECS implementation.
- UUIDs: rejected for the spike because 128-bit identity doubles key size without a current distributed-generation requirement.
- Recycle deleted IDs: rejected because old events and historical references must never resolve to a different entity.
- Encode entity kind into ID bits: deferred because it reduces ID space and hard-codes a taxonomy before the entity model is validated.

