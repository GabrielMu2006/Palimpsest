# CHRON-008 — 10K Dummy Entity Benchmark

## Context
Phase 0 must measure whether standalone `bevy_ecs` is a viable runtime ECS hypothesis while persistent identity remains separate.
## Scope
Benchmark 100/1K/3K/5K/10K dummy entities, stable-ID-to-runtime mapping, update throughput, and process RSS.
## Out of Scope
Full NPC AI, official 10K gameplay claims, LOD policy, and optimization that removes identity/history boundaries.
## Dependencies
CHRON-004, CHRON-006, and CHRON-007.
## Files Modified / Allowed
Workspace/headless manifests; benchmark binary; `docs/PERFORMANCE.md`; ADR-0005; benchmark report; this task.
## API Contract
`EntityId` remains persistent; `bevy_ecs::Entity` is held only in a runtime lookup map and never serialized.
## Tests
Exact entity/mapping counts, update counts, stable mapping resolution, workspace checks.
## Benchmark
Release runs at 100, 1K, 3K, 5K, 10K with 1,000 full-update steps; process RSS baseline and 10K delta.
## Definition of Done
M5 16GB measurements and limitations are recorded, identity separation is proven, and ADR-0005 gives a measured Phase 0 recommendation.
