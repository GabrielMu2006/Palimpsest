# ADR-0017: Phase 1 Crate Boundaries

- Status: Proposed — awaiting product-owner approval with CHRON-018
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for cross-crate boundary changes

## Context

ADR-0001 requires focused crates to be added only when an independently testable
boundary is needed. Phase 1 introduces world/tile domain state and explainable
Utility AI. Putting both directly into the composition root would make the
headless boundary harder to test and would couple spatial primitives to ECS
orchestration.

## Decision

Add two Godot-independent library boundaries when CHRON-018 is approved:

- `palimpsest-sim-world`: `WorldGrid`, `LocalGrid`, coordinates, terrain,
  deterministic world generation, Activity Sites, and deterministic local-grid
  pathfinding.
- `palimpsest-sim-ai`: Needs, action/decision contracts, DecisionTrace, and
  Utility scoring/selection.

`palimpsest-sim-core` remains the headless composition root. It owns runtime ECS
integration, Person runtime mapping, action execution, kernel orchestration,
commands, worker ownership, and Render Snapshot construction.

Dependency direction:

```text
sim-entity / sim-time
          ↑
       sim-world
          ↑
        sim-ai
          ↑
       sim-core
          ↑
headless-runner / godot-bridge
```

`sim-world` and `sim-ai` must not depend on `bevy_ecs`, storage, Godot, or an LLM
runtime. `sim-core` may use `bevy_ecs` provisionally under ADR-0011.

## Public Contract

- Domain crates expose stable domain values and pure/deterministic operations.
- Runtime ECS handles and worker/thread types remain internal to `sim-core`.
- `godot-bridge` translates Render Snapshots and commands only; no simulation
  crate depends on it.
- Phase 1 adds no speculative empty crates beyond these two boundaries.

## Consequences

- World/pathfinding and Utility calculations can be tested without ECS or Godot.
- `sim-core` remains the only composition layer for runtime mutation.
- Cross-crate APIs must stay minimal; an implementation that needs a dependency
  outside this graph requires ADR review before code changes.

## Rejected / Deferred Alternatives

- Put all Phase 1 code in `sim-core`: rejected because it weakens independent
  domain testing and creates a large composition crate.
- Create separate crates for Person, Needs, pathfinding, actions, and rendering:
  rejected as premature fragmentation for the small Phase 1 kernel.
- Let `sim-ai` own pathfinding: rejected; pathfinding is a spatial world service
  used by AI, not an AI decision contract.

## Supersedes / Extends

Extends ADR-0001 with the first gameplay-domain crate split. It does not change
ADR-0007 or authorize Phase 1 implementation without Task approval.
