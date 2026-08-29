# ADR-0006: Structured Event Schema

- Status: Accepted for Architecture Spike
- Date: 2026-08-29

## Context
History, storage, replay, knowledge, and future narrative systems require structured causal facts rather than prose.

## Decision
Use a versioned `EventRecord` with stable `EventId`, `SimInstant`, type key, `EntityId` actor/target/location references, `EventId` causes/consequences, visibility, bounded 0–1000 significance, and ordered JSON metadata. Validate on deserialization. Runtime ECS/Godot identities and narrative text are forbidden.

## Consequences
Events are queryable, causally linkable, and presentation-independent. Metadata remains extensible but should not replace typed fields when a concept stabilizes. Schema evolution requires migration.

## Alternatives Considered
- `Vec<String>`: rejected as non-queryable and non-causal.
- ECS entities: rejected as runtime-local.
- Generated prose as truth: rejected by Simulation First.

