# CHRON-015 — Test, Lint, Benchmark, and CI

## Context
Phase 0 needs repeatable quality gates for the headless Rust core and macOS Godot-Rust integration.
## Scope
Pin Rust, initialize local Git metadata, add local CI entry points, add GitHub Actions jobs for Rust quality/MSRV/benchmark smoke checks and Godot macOS integration, and validate them locally.
## Out of Scope
Publishing releases, signing/notarization, deployment, performance regression thresholds from heterogeneous CI hardware, and repository hosting administration.
## Dependencies
CHRON-003, CHRON-007, CHRON-008, CHRON-009, CHRON-011, CHRON-012, CHRON-013, and CHRON-016.
## Files Modified / Allowed
Git metadata, toolchain file, `.github/workflows/**`, `tools/ci-*.sh`, this task, and tooling/performance documentation.
## Tests
Run both local entry points, validate the workflow YAML, verify MSRV 1.95, and ensure the read-only Master Spec hash gate passes.
## Benchmark
CI executes correctness-preserving smoke workloads for Scheduler, 10K ECS, events, SQLite, snapshots, and the headless runner. Published performance claims remain M5-local release measurements.
## Definition of Done
All local gates pass and a GitHub Actions workflow is ready to run after the repository is pushed to a GitHub remote; no tests are skipped or weakened. **Complete locally; hosted execution awaits a remote push.**
