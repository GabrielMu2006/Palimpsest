# CHRON-013 — SQLite Event Store Prototype

## Context
Phase 0 must test durable structured-event storage with SQLite WAL and snapshots later.
## Scope
SQLite schema, WAL/NORMAL/foreign keys, atomic batch append, causal edges, indexed queries, checkpoint, reopen, and integrity tests.
## Out of Scope
Final retention, archive tables, `.world` packaging, snapshots, and migration policy.
## Dependencies
CHRON-009.
## Files Modified / Allowed
Workspace manifest/lock; `crates/sim-storage/**`; ADR-0008; benchmark report; performance docs; this task.
## API Contract
Validated EventRecord JSON is authoritative payload; IDs/timestamp/type are indexed; batches are atomic; IDs must fit SQLite signed INTEGER.
## Tests
Append/get/count, duplicate rollback, WAL checkpoint, reopen, integrity, workspace checks.
## Benchmark
100K events across documented batch sizes under WAL + synchronous NORMAL.
## Definition of Done
Events survive reopen, failed batches roll back, integrity passes, and exact throughput/settings/DB growth are reported.

