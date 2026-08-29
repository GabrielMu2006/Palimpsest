# CHRON-021 — Person Runtime Model

> **Status: Complete — awaiting product-owner confirmation.**
> The product owner approved this single Task on 2026-08-29; implementation stayed within the Files Modified / Allowed boundary.

## Objective
Establish the minimal runtime Person shell in Simulation Core: a stable `EntityId` bound to a runtime ECS handle plus a valid `Location`. Needs and CurrentAction are attached by later Tasks, avoiding a dependency cycle.

## Context
ADR-0002 requires persistent identity (`EntityId`) to be independent of runtime ECS handles; ADR-0011 keeps `bevy_ecs` provisional with a separate, non-persistent `EntityId -> bevy_ecs::Entity` map. Phase 1 first needs a Person identity/location shell so CHRON-022 can attach Needs and CHRON-025/027 can attach and execute actions without creating a dependency cycle. This Task excludes all Phase-2 personality/social/body data.

## Scope
- Add a headless Person runtime model in `sim-core` (the composition root) using the existing stable `EntityId` (from `sim-entity`) and the runtime handle/mapping approach.
- Maintain a non-persistent runtime mapping `EntityId -> runtime ECS handle` (or an equivalent runtime index) with a documented policy: it is rebuilt at startup, is never serialized, and is not part of persistence (ADR-0002).
- Define only `Person` marker, stable `EntityId`, and integer tile `Location(LocalCoord)` components. No sub-tile remainder or movement physics is introduced.
- Provide a `spawn`/`insert` path that pairs a freshly allocated `EntityId` with a new runtime handle and attaches these components; expose read/update access for Location through stable identity.
- Provide a bounded, headless API (no Godot) usable by the kernel (CHRON-028), the headless runner (CHRON-007), and later the state machine (CHRON-027).
- Keep the runtime model deterministic and free of `bevy_ecs`-handle leakage into any `EntityId`-based API or persistence contract.

## Out of Scope
- Personality, Values, Skills, Profession, Family, Relations, Memory, Knowledge, Goals, Beliefs, Inventory, Body/Health, or any Phase 2 person depth.
- Needs component/model (CHRON-022), CurrentAction/action contracts (CHRON-025), or Utility scoring (CHRON-026).
- Pathfinding (CHRON-024) and movement stepping logic; this Task only stores a location and action.
- Persistence/serialization of the runtime map, snapshots, or save format.
- Godot bridge, rendering, input, UI.
- LLM, NLG, war, politics, religion, magic, historians.

## Dependencies
- CHRON-018 (`sim-world`/`sim-ai` boundaries) and CHRON-019 (`LocalCoord`/`LocalGrid`) complete.
- CHRON-004 (`EntityId`, `EntityIdAllocator`) and ADR-0002/ADR-0005 (runtime mapping policy) complete.
- CHRON-006 Scheduler is not required by this Task but may be referenced for scheduling location changes later.

## Files Modified / Allowed
- `crates/sim-core/**` — the **existing** composition-root crate. This Task adds a `person` runtime module (e.g. `src/person.rs`) and re-exports the runtime Person API from `src/lib.rs`.
- `crates/sim-core/Cargo.toml` — add `palimpsest-sim-world` and `bevy_ecs`; do not add `sim-ai` until CHRON-022.
- `Cargo.toml`, `Cargo.lock` — update workspace deps/members as required by the above.
- `docs/adr/ADR-0011-phase-1-runtime-ecs.md`, `ADR-0013-person-needs-action-boundaries.md`, and `ADR-0017-phase-1-crate-boundaries.md`; no new ADR is needed unless implementation deviates.
- `docs/tasks/CHRON-021.md`.
- No other file; do not modify `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` without a genuine-conflict Change Proposal first.

## API Contract
- A headless `PersonRuntime`-style type (or module) exposing:
  - `spawn(allocator, world, location) -> Result<EntityId, ...>` — allocates a stable `EntityId`, creates a runtime handle, and attaches Person + Location.
  - `get(world, id: EntityId) -> Option<PersonView>` — read-only stable identity and location only.
  - `set_location` — an explicit, checked tile-location update.
  - `runtime_handle(id) -> Option<...>` — the runtime handle is exposed only inside `sim-core` (or a `#[doc(hidden)]` diagnostics path) and is not part of the public domain API that crosses persistence/bridge boundaries.
  - `location(id)` — the accessor used by later systems and Developer Mode.
- Invariants to record:
  1. A person's persistent `EntityId` is unique and never reused within a world; the runtime handle is a rebuildable, non-persistent binding.
  2. The runtime mapping is never serialized; only `EntityId` (plus domain components' own serde) crosses persistence.
  3. `Location` is always a valid in-bounds `LocalCoord` (correctness test), or the runtime guarantees a documented invalid/`None` representation rather than a panic.
  4. Needs and CurrentAction are deliberately absent until CHRON-022/025.
  5. The runtime model is headless and independent of Godot and LLM.

## Tests
- Stable-identity pairing: spawning N people produces N unique `EntityId`s; each resolves to its own runtime handle; no handle is reused.
- Mapping is non-persistent: the runtime map/`PersonRuntime` contains no `EntityId`-or-handle field that is serde-serializable as a persistent identity (compiled-inspection test or explicit no-Serialize test); handles are not reachable through the public `EntityId` API.
- Spawn/location: a freshly spawned person has the supplied valid Location; `location(id)` reflects `set_location`; unknown IDs leave state unchanged.
- Bounds safety: `Location` cannot be set to an out-of-bounds coordinate via the public API (returns error or clamps per documented rule); no panic is possible from a normal public call.
- Determinism: two identical sequences of spawns + component updates produce identical visible state and identical consumed `EntityId` sequence.
- Workspace gates: fmt, Clippy with warnings denied, workspace tests, docs, dependency audit.

## Benchmark
- Headless person spawn + attach throughput at 100 and 1,000 persons, release build, ten post-warm-up samples, median reported on the M5 16 GB reference machine.
- Report spawn/s, per-person RSS delta, and (if measured) runtime-map memory; correctness assertions remain enabled.
- This is the Phase 1 person baseline feeding CHRON-028's 100-person kernel; do not claim 10K scale here (that belongs to a later scale gate).

## Definition of Done
- A headless Person runtime model in `sim-core` pairs a stable `EntityId` with a non-persistent runtime handle and attaches only Person marker + Location.
- The runtime mapping is never serialized; only `EntityId` and domain components cross persistence/bridge boundaries; no runtime handle is reachable through the public domain API.
- A person has a valid in-bounds Location; all public mutations are explicit and bounded. Needs and CurrentAction remain absent in this Task.
- No Phase 2 person depth (body, personality, values, skills, profession, family, relations, memory, goals, beliefs, inventory) is implemented.
- The runtime model is deterministic, headless, Godot-free, and LLM-free; its 100/1,000-person spawn benchmark is reproducible and documented.
- The runtime Person component set + runtime-handle policy conforms to ADR-0011/0013.

## Required Completion Report
Report: the exact change summary; the commands actually run; the spawn benchmark result (spawn/s, RSS delta) or explicit N/A; the list of covered identity/mapping/location/bounds test cases; known limitations (Needs and CurrentAction intentionally absent, no Phase 2 depth, runtime map non-persistent, bevy_ecs provisional); and any blocker. Do not auto-start the next Task; each Phase 1 Task requires separate product-owner approval.
