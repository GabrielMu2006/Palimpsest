# ADR-0009: Phase 0 Snapshot Format

- Status: Accepted for Architecture Spike
- Date: 2026-08-29

## Context
Historical replay and fast loading need snapshots, but ECS runtime handles and heap internals are not stable persistence.

## Decision
Use an 8-byte Palimpsest magic header followed by zstd level-3 compressed bincode-v2 Serde data. Snapshot schema v1 stores SimClock, EntityId allocator progress, stable entity DTOs, and stable pending-work DTOs reconstructed into runtime systems. Validate before encoding and after decoding.

## Consequences
Snapshots are compact and headless but not yet a permanent save format. Schema migration, decompression limits, deltas, and content versions must be added before untrusted or long-term save support.

## Alternatives Considered
- Serialize Bevy World/Entity: rejected as runtime coupling.
- JSON snapshots: rejected for size/performance baseline, though useful for debugging.
- Uncompressed bincode: retained only as a measured intermediate, not the stored artifact.
