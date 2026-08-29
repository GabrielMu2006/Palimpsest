---
name: palimpsest-rust-sim
description: Implement or review Palimpsest's Rust Simulation Core. Use when working on Rust core entities, schedulers, simulation systems, serialization boundaries, or headless execution; do not use for Godot-only UI work.
---

# Palimpsest Rust Sim

Read `MASTER_SPEC.md`, `AGENTS.md` when present, and relevant ADRs and task specs. Preserve these core invariants:

- Use stable persistent `EntityId` values; never persist runtime ECS handles as identity.
- Prefer scheduled or event-driven work; do not scan every entity every tick.
- Keep ownership clear, errors explicit, and global mutable state absent.
- Define serialization and persistence boundaries explicitly.
- Keep the core runnable headlessly and independent of Godot and LLMs.
- Require an ADR for public API or architectural changes.

Measure before optimizing. For each change, consider and run the relevant subset of `cargo fmt`, `cargo clippy`, `cargo test`, and benchmarks. Report anything unavailable. Do not hard-code crate versions in this skill.
