# CHRON-016 — Event Throughput Benchmark

## Context
Phase 0 must measure structured-event creation and serialization separately from storage.
## Scope
Benchmark validated event generation, JSON serialization, total bytes, and bytes/event on M5 16GB.
## Out of Scope
SQLite, NLG strings, final gameplay event mix, and retention.
## Dependencies
CHRON-009.
## Files Modified / Allowed
Event benchmark binary; performance report/docs; this task.
## API Contract
Benchmark uses the production EventRecord validation and serialization path.
## Tests
Exact event counts, valid IDs and actors, successful serialization, workspace checks.
## Benchmark
100K representative events, ten samples after warm-up; min/median/max and process RSS.
## Definition of Done
Generation and serialization events/s are reported separately with payload size and limitations; no SQLite claim is inferred.
