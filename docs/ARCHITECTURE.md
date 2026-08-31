# Palimpsest Architecture

`MASTER_SPEC.md` is authoritative. This document describes the spike baseline and current Phase1 boundaries; it may not override the specification.

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

CHRON-031 adds the micro-world presentation path (ADR-0026):
`PalimpsestMicroWorld` owns the CHRON-030 worker; one batched `snapshot_frame()`
per rendered frame carries the schema-2 snapshot (terrain bytes, site triples,
per-person arrays) plus worker metrics, with stable `EntityId` values encoded
losslessly as 8 little-endian bytes (never through `f64`/Godot `int`).
Commands (`pause`/`resume`/`set_speed`/`step`/`advance_to`/`shutdown`) enter
only through the bounded worker queue; enqueue failure and the later
applied/rejected acknowledgement are distinct states. Conversion is a pure,
engine-free module plus a thin `#[func]` layer. The windowed 100-person client
holds 60 FPS (120 warm-up + 300 measured frames; see `docs/PERFORMANCE.md`).

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

CHRON-018 established two boundary-only crates ahead of Micro World Kernel
implementation. At that historical point they contained crate documentation
and no domain logic; CHRON-019..026 have since populated them, and this review
does not roll back those implementations.

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
`sim-ai` may add `sim-world` to that set. These are allow-sets, not mandatory
dependencies for an empty crate or for CHRON-018's historical skeletons.
All simulation/domain crates must not depend outward on `godot-bridge`, which
is an outer adapter. The additional allow-set exclusions (`sim-core`,
`sim-events`, `sim-scheduler`, `sim-storage`, `bevy_ecs`, Godot, and an LLM
runtime) apply specifically to `sim-world` and `sim-ai`. The current graph is reviewed using exact normal dependency sets from
Cargo metadata plus `cargo tree --edges normal`, at the workspace CI/lint gate
and whenever dependencies change. This replaced the removed custom audit
integration test; it is more precise about the present graph but does not
provide automatic future enforcement. `sim-core` remains the headless
composition root and gains its `sim-world`/`sim-ai` dependencies in CHRON-021.
CHRON-027 adds the action execution state machine there (ADR-0021): it owns
per-person execution records, the due-time/FIFO scheduler driving them,
bounded outcome events, and the critical-need check cadence; it never scores
or selects. See ADR-0017.
CHRON-028 lands the `WorldKernel` there (ADR-0022): the sole owner of time and
ordering, it composes the clock, static world, sites, person runtime, action
runtime, decision weights, latest per-person decision trace, and a bounded
outcome-event buffer, exposing a bounded `advance_to(target, work_budget)` that
jumps between due instants and reports the last committed boundary. CHRON-029
lands the immutable, versioned `RenderSnapshot` DTO there (ADR-0023): the
read-only presentation contract built strictly from the kernel's committed
boundary, carrying only stable `EntityId`, terrain batches, per-person action
state, and observable metrics — never ECS handles or scheduler tokens.
ADR-0024 repairs both: side-effect-free `start`/`cancel` rejection, merged
per-`(person, instant)` decision requests, a `Setup`/`Running`/`Faulted`
lifecycle with a `Result` read API and fallible snapshot builder, read-only
Needs projection, cumulative event total/digest with two-buffer rotation
accounting, and render schema 2 with the static activity-site batch and full
DTO validation.

ADR-0025 completes that contract: action timestamps include a separate last
successful transition watermark; Kernel queue metrics are cached at complete
boundaries and carry lifecycle/failure markers; `next_due` and `sites` are
fallible (sites also contain WorkCounter truth); projection errors are explicit.
Per-call event totals include rotated records. Terrain/person DTOs validate
independently as well as through the root schema-2 snapshot. Current acceptance
and historical reading routes live in `CURRENT_PROGRESS.md` and `TASK_INDEX.md`.

CHRON-030 lands the simulation worker there (ADR-0015 Phase 1 supplement): one
dedicated `std` thread owns the `WorldKernel` and is the only mutator; a
bounded 64-command queue carries `Pause`/`Resume`/`SetSpeed(1/5/20/100/1000/
MAX)`/`Step`/`AdvanceTo`/`Shutdown` with monotonic sequences and a bounded
1,024-ack log; immutable schema-2 snapshots publish between kernel calls (one
exchange slot, forced on pause/step/advance/shutdown, throttled to 10 Hz while
running). Speed changes wall-clock pacing only; faults retain the last complete
publication; an independent atomic stop path closes the worker even with a full
queue. Safe Rust, standard library only; no IPC, process, or parallel ECS.

## Benchmark-Only Memory Instrumentation

REM-008A adds `tools/bench-memory` as an outward-only workspace binary. It
imports the headless domain libraries and reuses example fixtures, but no
simulation crate depends on it. Original example timing paths have no memory
observer or platform call. ADR-0020 records its private macOS native RSS-read
and fixed-size diagnostic mmap/munmap exceptions; production crates continue
to inherit `unsafe_code = forbid`. There is no domain, persistence, bridge,
allocator or ECS-contract change. Cold and prepared-operation peak RSS are
separate measurements with checked interval provenance, not simulation state.

## Headless 10-Year Chaos Runner (CHRON-032, ADR-0027)

CHRON-032 adds a headless correctness gate that drives the `WorldKernel`
directly for the full Phase 1 horizon (10 × 365-day years = `315_360_000` s)
from a fixed seed. The runner is an **outer driver only** and never mutates
world state: it never teleports a person, never manufactures a selection, and
writes nothing back. Spawn cells are resolved deterministically from the seed-42
map to a connected walkable component that contains a Meal, a Rest, and a Work
site, so every person has a real (pathfinding) route to each. The instrument —
bounded per-day invariant checks (needs in `[0, 100_000]`, queue depth ≤ 2×
population, buffer `total = delivered + buffered + rotated`, every drained event
actor resolving, strict progress) and a `NonTerminating`/`Watchdog` liveness
guard — lives in the Core (`chaos` module) and returns a typed `ChaosError`
(non-zero bin exit) on the first violation. A canonical FNV-1a-64 truth hash
covers config + time + ordered per-person views + sites + kernel counters and
excludes wall-clock/RSS/thread/pointer/ECS handles, so same-seed runs are
byte-deterministic. The binary `chaos_runner` in `headless-runner` is a thin
parse/report wrapper; the Report (JSON + Markdown) feeds CHRON-033/034/036.

## Phase1 closeout

WorldKernel remains authoritative and headless. A single in-process worker owns
it; Godot consumes immutable render snapshots and acknowledged commands. Paired
publication/status observation, real movement accounting and diagnostic-only path/
scheduler counters are recorded in ADR0028/0029. Wall time never decides truth.
The shared production spike API is retired (ADR0010); the default headless runner
uses the actual kernel. Primitive benchmarks/historical reports are retained as
such, not current gameplay measurements.

[The Phase1 report](reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md) recommends continuing
bevy_ecs provisionally and the current worker for100persons.10KCore diagnostics
are not a10Kclient/full-lifecycle guarantee. Rolling diagnostics do not replace
durable EventStore/history, render DTOs do not become production saves, and all
LOD/identity/history/persistence/optional-LLM boundaries remain. Phase2 needs a
new explicit approval; report delivery does not authorize it.
