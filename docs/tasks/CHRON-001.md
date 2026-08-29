# CHRON-001 — Rust Workspace

## Context

Phase 0 needs a Rust workspace that can host a Godot-independent simulation core. This task establishes only the build boundary and quality baseline.

## Scope

- Create a Cargo workspace using Rust 2024 and resolver 3.
- Install the minimal stable Rust toolchain needed to verify the workspace when it is absent.
- Add an intentionally minimal `palimpsest-sim-core` library crate.
- Centralize baseline Rust and Clippy lints.
- Record the initial workspace dependency direction in ADR-0001.
- Document the Phase 0 repository plan and task DAG.
- Establish the minimal repository-wide task, ADR, and proposal governance files required before later Phase 0 work.

## Out of Scope

- `EntityId`, `SimClock`, Scheduler, events, persistence, snapshots, benchmarks, or simulation systems.
- Godot project files or GDExtension integration.
- NPC AI, content systems, LLM/NLG, Rule Editor, or web code.
- CI implementation, Git repository initialization, dependency selection, or performance claims.

## Dependencies

- `MASTER_SPEC.md` (read-only).
- A Rust toolchain with Cargo, rustfmt, and Clippy for verification.

## Files Modified

- `Cargo.toml`
- `Cargo.lock`
- `.gitignore`
- `rust-toolchain.toml`
- `rustfmt.toml`
- `AGENTS.md`
- `crates/sim-core/Cargo.toml`
- `crates/sim-core/src/lib.rs`
- `docs/ARCHITECTURE.md`
- `docs/PHASE_0_PLAN.md`
- `docs/TOOLING.md`
- `docs/adr/ADR-0001-rust-workspace-boundaries.md`
- `docs/tasks/CHRON-001.md`
- `docs/tasks/TEMPLATE.md`
- `docs/proposals/TEMPLATE.md`

## API Contract

No public domain API is introduced. `palimpsest-sim-core` is a headless library root and must not depend on Godot.

## Tests

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`
- `cargo metadata --no-deps --format-version 1`

## Benchmark

Not applicable. This task adds no runtime behavior and makes no performance claim.

## Definition of Done

- Cargo resolves the workspace and recognizes `palimpsest-sim-core`.
- Formatting, lint, and test commands pass without skipped or weakened tests.
- The core crate has no Godot or LLM dependency.
- The workspace boundary is recorded in an ADR.
- No later Phase 0 system is implemented.
