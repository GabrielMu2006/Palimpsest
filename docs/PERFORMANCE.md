# Palimpsest Performance Baseline

`MASTER_SPEC.md` defines the M5 16GB performance contract. This document records methods and measured results; it cannot relax product budgets.

## Reference Machine

- Apple M5, 10 CPU cores
- 16 GB unified memory
- arm64 macOS
- Rust 1.98.0 stable

Machine serial numbers and other device identifiers are intentionally excluded.

## Measurement Rules

- Use release builds for throughput and latency claims.
- Record workload, sample count, warm-up, exact command, and relevant dependency versions.
- Keep correctness assertions enabled in benchmark harnesses.
- Report median and limitations; do not treat one noisy CI run as a performance gate.
- Scale entity workloads through 100, 1K, 3K, 5K, and 10K where applicable.
- Preserve identity, causality, history fidelity, and tests during optimization.

## Results

Task-specific raw results live under `docs/reports/`. Architecture-wide conclusions belong only in `docs/reports/ARCHITECTURE_SPIKE_V1.md` after all Phase 0 measurements complete.

- CHRON-006 Scheduler: `docs/reports/CHRON-006_SCHEDULER_BASELINE.md`
- CHRON-008 10K ECS dummy entities: `docs/reports/CHRON-008_10K_DUMMY_BENCHMARK.md`
- CHRON-016 structured-event throughput: `docs/reports/CHRON-016_EVENT_THROUGHPUT.md`
- CHRON-013 SQLite Event Store: `docs/reports/CHRON-013_SQLITE_EVENT_STORE.md`
- CHRON-012 Snapshot prototype: `docs/reports/CHRON-012_SNAPSHOT_PROTOTYPE.md`
- CHRON-003 Godot-Rust bridge: `docs/reports/CHRON-003_GODOT_RUST_BRIDGE.md`
- CHRON-011 128×128 Tile renderer: `docs/reports/CHRON-011_TILE_RENDERER.md`
