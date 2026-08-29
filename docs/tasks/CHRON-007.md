# CHRON-007 — Headless Runner

## Context
Phase 0 must prove the Simulation Core runs finitely and observably without Godot.
## Scope
Create a CLI that composes stable IDs, clock, Scheduler, dummy work, structured events, and JSON metrics.
## Out of Scope
ECS, NPC AI, gameplay world generation, storage, snapshots, and rendered mode.
## Dependencies
CHRON-004, CHRON-005, CHRON-006, and CHRON-009.
## Files Modified / Allowed
Workspace manifest; `apps/headless-runner/**`; this task document.
## API Contract
Finite inputs produce deterministic domain metrics on stdout; invalid CLI/domain inputs use non-zero exit status and stderr.
## Tests
10K deterministic fixture, invalid time, CLI success/failure smoke, workspace checks.
## Benchmark
Release wall-time baseline for the 10K dummy fixture; full ECS throughput belongs to CHRON-008.
## Definition of Done
Runner terminates without Godot, outputs valid JSON, drains due work, creates validated events, and reports failures explicitly.
