# ADR-0001: Rust Workspace Boundaries

- Status: Accepted for Architecture Spike
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for changes to this boundary

## Context

Palimpsest requires a Rust simulation core that runs without Godot, while the macOS client consumes a narrow bridge API. Phase 0 must validate this split without prematurely defining all future module APIs.

## Decision

Use a virtual Cargo workspace. Begin with one headless library crate, `palimpsest-sim-core`. Add focused crates only when their scoped Phase 0 tasks require an independently testable boundary.

Dependency direction is inward:

1. Domain and simulation crates must not depend on Godot.
2. Storage may depend on stable domain serialization contracts, never on client state.
3. `godot-bridge` may depend on simulation-facing crates; simulation crates must not depend on it.
4. Headless tools may depend on simulation and storage crates; they cannot be required by the core library.
5. LLM libraries are not part of the Simulation Core dependency graph.

The workspace uses Rust 2024, Cargo resolver 3, workspace lint inheritance, and forbids unsafe Rust unless a later ADR explicitly changes that policy for a narrowly justified boundary.

## Consequences

- Headless compilation is the default architecture check.
- Godot integration remains replaceable and cannot own simulation truth.
- Crate boundaries will be introduced incrementally, avoiding speculative fragmentation.
- A later task may choose `bevy_ecs`, but this ADR does not decide that spike outcome.

## Alternatives Considered

- One crate for the entire project: rejected because it weakens the Godot/core dependency boundary.
- Create every future crate immediately: rejected because empty speculative modules would lock in unvalidated boundaries.
- Make Godot the application root for simulation: rejected because it conflicts with the Master Spec.

