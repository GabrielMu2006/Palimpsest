# ADR-0007: Godot GDExtension Boundary

- Status: Accepted for Architecture Spike
- Date: 2026-08-29

## Context
Godot must consume Rust render snapshots without becoming simulation truth. godot-rust requires an unsafe ExtensionLibrary marker at native registration.

## Decision
Use a dedicated `cdylib` adapter depending inward on sim-core. Expose presentation-only Godot classes and view-model methods. Simulation crates never depend on Godot. Permit unsafe code only in this crate and initially only for the required `unsafe impl ExtensionLibrary`; document the safety boundary inline. Batch future render data rather than exposing per-entity authoritative mutation.

## Consequences
Core stays headless and reusable. Godot can reconstruct presentation from snapshots. The bridge inherits native ABI/build complexity and needs per-platform libraries. Any additional unsafe block requires ADR review.

## Alternatives Considered
- Godot owns simulation Nodes: rejected by Master Spec.
- IPC process boundary: deferred; unnecessary complexity for the initial macOS spike.
- C ABI handwritten bridge: rejected while godot-rust supplies typed integration.
