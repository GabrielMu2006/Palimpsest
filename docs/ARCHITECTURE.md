# Palimpsest Architecture — Phase 0 Baseline

`MASTER_SPEC.md` is authoritative. This document describes the Architecture Spike baseline and may not override it.

## System Boundary

```text
Godot macOS client
  UI / input / tile rendering / metrics overlay
             |
       GDExtension bridge
             |
Rust headless Simulation Core (authoritative)
             |
 structured events / snapshots / SQLite prototypes
```

The Godot Scene Tree is presentation state, never simulation truth. Persistent identity uses a stable domain `EntityId`; runtime ECS handles are replaceable runtime indexes. LLM functionality is absent from Phase 0 and can never be required for simulation.

## Recommended Initial Repository

```text
Palimpsest/
├── apps/
│   ├── headless-runner/
│   └── macos-godot/
├── crates/
│   ├── sim-core/
│   ├── sim-entity/
│   ├── sim-time/
│   ├── sim-events/
│   ├── sim-storage/
│   ├── sim-debug/
│   └── godot-bridge/
├── benchmarks/
├── content/
├── docs/
│   ├── adr/
│   ├── proposals/
│   ├── reports/
│   └── tasks/
├── tests/
│   ├── regression/
│   ├── simulation/
│   └── worlds/
└── tools/
```

Directories and crates are created only when their task needs them. Phase 0 must not pre-build Phase 1 gameplay systems.

## Initial Dependency Rules

- `sim-core` composes simulation facilities and remains headless.
- `sim-entity`, `sim-time`, and `sim-events` expose domain primitives without Godot dependencies.
- `sim-storage` persists stable domain representations and never persists ECS runtime handles.
- `sim-debug` exposes read-only metrics and diagnostics.
- `godot-bridge` translates immutable/render-oriented snapshots for the client.
- `apps/headless-runner` and `apps/macos-godot` are adapters at the outer edge.

See ADR-0001 for the recorded decision.

## Persistent Identity

`palimpsest-sim-entity::EntityId` is the canonical identity carried by events,
storage, snapshots, history, and client-facing view models. It is a non-zero
`u64`, allocated monotonically and never recycled. Runtime ECS handles remain in
a separate, non-persistent lookup layer that will be selected and measured in a
later Phase 0 task. See ADR-0002.

## Simulation Time

`palimpsest-sim-time` defines signed integer-second `SimInstant` values,
non-negative `SimDuration` values, and a monotonic `SimClock`. Simulation time
is independent of wall-clock time, Godot frames, and execution speed. Arithmetic
is checked and persisted as numeric seconds. See ADR-0003.

## Scheduling

`palimpsest-sim-scheduler` owns a deterministic due-time priority queue. It
returns due payloads to headless callers and never invokes system callbacks or
scans entities internally. Equal-time work is FIFO; runtime cancellation tokens
are not persistent identity. Queue health is exposed for Developer Metrics. See
ADR-0004.

## Structured Events

`palimpsest-sim-events` defines versioned causal records using stable `EventId`,
`EntityId`, and `SimInstant` references. Event truth is structured; prose,
beliefs, claims, and historiography remain separate. See ADR-0006.

## Runtime ECS Spike

Phase 0 continues with standalone `bevy_ecs` 0.19.1 based on measured 10K dummy
results. Persistent `EntityId` remains a component and maps to runtime
`bevy_ecs::Entity` values through a non-persistent lookup. See ADR-0005.

## Persistence Prototypes

Structured events use SQLite WAL with atomic batch append and checkpointing.
Domain snapshots use versioned bincode data compressed with zstd; they persist
stable IDs, clock, allocator progress, and reconstructable pending work rather
than ECS handles or heap internals. See ADR-0008 and ADR-0009.

## Godot Bridge

`palimpsest-godot-bridge` is the only crate that depends on godot-rust. It is a
presentation adapter: Godot requests immutable render-oriented dictionaries and
cannot mutate Simulation Core state. The only unsafe declaration is the
godot-rust-required `ExtensionLibrary` registration marker; workspace simulation
crates retain `unsafe_code = "forbid"`. See ADR-0007.

## Developer Metrics

The first Godot overlay is a read-only observer. It combines Godot performance
monitors, TileMapLayer benchmark state, bridge health, and fields copied from the
Rust Render Snapshot. It exposes no simulation mutation controls and labels
unavailable client-side scheduler state rather than inventing Godot-owned truth.

## Phase 0 Shared Workload

`sim-core::run_spike_workload` is a temporary deterministic workload shared by
the standalone runner and Godot bridge solely to compare process modes with
identical Rust code. It is not a game-system API and must be reviewed or removed
before Phase 1. See ADR-0010.

## Phase 1 Crate Plan

CHRON-018 added two boundary-only crates ahead of Micro World Kernel
implementation. They contain crate documentation and no domain logic; later
tasks populate them.

- `palimpsest-sim-world` hosts local-grid coordinates, terrain, deterministic
  world generation, activity sites, and deterministic local-grid pathfinding
  (CHRON-019, CHRON-020, CHRON-023, CHRON-024).
- `palimpsest-sim-ai` hosts needs, action/decision-trace contracts, and
  utility scoring/selection (CHRON-022, CHRON-025, CHRON-026).

The Phase 1 dependency direction extends the Phase 0 inward rule:

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

`sim-world` may depend only on `sim-entity`, `sim-time`, and `serde`;
`sim-ai` may add `sim-world` to that set. Neither may depend on `sim-core`,
`sim-events`, `sim-scheduler`, `sim-storage`, `godot-bridge`, `bevy_ecs`,
Godot, or any LLM crate. The allow-sets are enforced by an audit integration
test in `crates/sim-ai/tests/dependency_direction.rs`. `sim-core` remains the
headless composition root and gains its `sim-world`/`sim-ai` dependencies in
CHRON-021. See ADR-0017.
