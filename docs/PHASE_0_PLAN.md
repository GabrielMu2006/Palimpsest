# Phase 0 — Architecture Spike Plan

## Goal Interpretation

Phase 0 proves that a Godot 4 macOS presentation client can efficiently consume render data from an authoritative, independently runnable Rust simulation. It validates persistent identity, scheduled time, structured event/history storage, snapshots, 10,000 dummy-entity scale, tile rendering, developer telemetry, and repeatable build/benchmark automation. It does not build gameplay content or enter Phase 1.

The terminal artifact is `docs/reports/ARCHITECTURE_SPIKE_V1.md`, which must contain measurements from the M5 16GB reference machine and explicit recommendations for Godot + Rust and `bevy_ecs`. Product-owner confirmation is required before Phase 1.

## Task DAG

```text
CHRON-001 Rust Workspace ─┬─> CHRON-004 Stable EntityId ─┬─> CHRON-009 Structured Event ─┬─> CHRON-016 Event Throughput
                         │                              │                                ├─> CHRON-013 SQLite Event Prototype
                         ├─> CHRON-005 Simulation Clock ├─> CHRON-006 Scheduler ─────────┼─> CHRON-007 Headless Runner
                         │                              │                                └─> CHRON-012 Snapshot Prototype
                         ├─> CHRON-002 Godot Project ───┴─> CHRON-003 Godot-Rust Bridge ──> CHRON-011 128x128 Tile Renderer
                         │                                                                └─> CHRON-010 Developer Metrics
                         └─> CHRON-015 CI

CHRON-004 + CHRON-006 + CHRON-007 ─> CHRON-008 10K Dummy Benchmark
CHRON-003 + CHRON-007 + CHRON-010 + CHRON-011 + CHRON-012 + CHRON-013 + CHRON-015 + CHRON-016
    ─> CHRON-014 Architecture Spike Report
```

Governance (`AGENTS.md`, ADR, proposal, task templates) is established alongside CHRON-001 and then applies to every task.

## Task Definitions and DoD

### CHRON-001 — Rust Workspace

- Scope: Cargo workspace, minimal headless core crate, lint/test baseline, repository architecture plan.
- Out of Scope: domain systems, third-party runtime dependencies, Godot, benchmarks.
- Dependencies: Master Spec; Rust toolchain for verification.
- Files: root Cargo/toolchain config, `crates/sim-core`, architecture/task/ADR docs.
- Tests: fmt, Clippy with warnings denied, workspace tests, Cargo metadata.
- Benchmark: N/A.
- DoD: workspace resolves and all checks pass; core has no Godot/LLM dependency; ADR-0001 exists.

### CHRON-002 — Godot 4 macOS Project

- Scope: minimal Godot 4 project and macOS launch scene.
- Out of Scope: Rust bridge, tile renderer, gameplay UI.
- Dependencies: supported Godot installation.
- Files: `apps/macos-godot/**`, task documentation.
- Tests: headless import/project validation and macOS launch smoke test.
- Benchmark: N/A.
- DoD: project opens without errors and displays a minimal empty client window.

### CHRON-003 — Godot-Rust Bridge

- Scope: GDExtension library, lifecycle boundary, one typed call returning a render snapshot payload, bridge overhead harness.
- Out of Scope: simulation ownership in Godot, full renderer, gameplay APIs.
- Dependencies: CHRON-001, CHRON-002, stable bridge-facing identity/time contract.
- Files: `crates/godot-bridge/**`, Godot extension config/loading code, ADR.
- Tests: Rust unit tests, Godot extension load smoke test, headless core remains buildable without Godot.
- Benchmark: batched and per-call bridge overhead with method and payload size recorded.
- DoD: Godot receives a Rust-produced snapshot; no simulation truth resides in Scene Tree; overhead is reported.

### CHRON-004 — Stable EntityId

- Scope: persistent `EntityId(u64)` semantics, allocation prototype, serialization and ECS mapping tests.
- Out of Scope: full entity model or NPC components.
- Dependencies: CHRON-001.
- Files: `crates/sim-entity/**`, integration wiring, ADR.
- Tests: uniqueness, round-trip serialization, invalid/reserved values, runtime-handle separation.
- Benchmark: allocation/lookup baseline if mapping is introduced.
- DoD: persisted data references only `EntityId`; no ECS handle crosses persistence/bridge boundaries.

### CHRON-005 — Simulation Clock

- Scope: `SimInstant`, monotonic advancement, checked arithmetic, speed-independent simulation time.
- Out of Scope: calendar lore and real-time UI controls.
- Dependencies: CHRON-001.
- Files: `crates/sim-time/**`, tests, ADR if serialization is fixed.
- Tests: boundaries, monotonicity, overflow handling, serialization round trip.
- Benchmark: N/A.
- DoD: deterministic integer-based time primitive compiles headlessly with explicit errors.

### CHRON-006 — Extensible Scheduler

- Scope: due-time queue, stable ordering policy, cancellation/rescheduling, metrics surface.
- Out of Scope: NPC AI and scanning all entities every tick.
- Dependencies: CHRON-004, CHRON-005.
- Files: scheduler module/crate, tests, ADR for public scheduling contract.
- Tests: ordering, equal-time tie behavior, cancellation, reentrancy policy, empty/large queue.
- Benchmark: enqueue/dequeue throughput and queue memory at representative sizes.
- DoD: systems execute only when due; behavior is deterministic under documented inputs; queue depth is observable.

### CHRON-007 — Headless Runner

- Scope: CLI runner composing clock, scheduler, entities, events, and machine-readable metrics.
- Out of Scope: Godot, gameplay world generation, 200-year content validation.
- Dependencies: CHRON-004, CHRON-005, CHRON-006, CHRON-009.
- Files: `apps/headless-runner/**`, fixtures/tests.
- Tests: CLI smoke, finite-run termination, deterministic fixture metrics, failure exit codes.
- Benchmark: headless execution rate baseline.
- DoD: simulation prototype runs to a requested simulated time without Godot and emits structured metrics.

### CHRON-008 — 10K Dummy Entity Benchmark

- Scope: staged 100/1K/3K/5K/10K dummy workloads, RSS and throughput methodology, `bevy_ecs` hypothesis measurement.
- Out of Scope: claims about full NPC AI scale.
- Dependencies: CHRON-004, CHRON-006, CHRON-007.
- Files: benchmark harness, fixtures, performance docs/reports.
- Tests: benchmark fixture correctness and entity-count assertions.
- Benchmark: RAM, simulation throughput, scheduler behavior for every scale gate.
- DoD: reproducible M5 16GB results include warm-up, samples, build profile, variance, peak RSS, and no budget relaxation.

### CHRON-009 — Structured Event

- Scope: typed event envelope with stable IDs, time, actors/targets, causes, consequences, location, visibility, significance, versioned metadata.
- Out of Scope: NLG, historiography, gameplay event catalog.
- Dependencies: CHRON-004, CHRON-005.
- Files: `crates/sim-events/**`, tests, event-model doc, ADR.
- Tests: schema invariants, stable-ID references, causality validation, serialization/version round trip.
- Benchmark: payload-size baseline.
- DoD: no event truth is stored as `Vec<String>`; schema is persistence-ready and versioned.

### CHRON-010 — Developer Metrics

- Scope: Rust metrics snapshot plus initial Godot overlay for TPS, entities, LOD counts, events/s, memory, DB size, scheduler queue and placeholder counters where systems do not exist.
- Out of Scope: Entity Inspector and fabricated metrics.
- Dependencies: CHRON-003, CHRON-006, CHRON-007; DB size after CHRON-013.
- Files: `crates/sim-debug/**`, bridge DTO, Godot overlay scene/scripts, tests.
- Tests: metric snapshot consistency, unavailable-field representation, overlay smoke test.
- Benchmark: metrics collection overhead.
- DoD: overlay updates without owning simulation state; unavailable Phase 0 systems are clearly marked, not invented.

### CHRON-011 — 128x128 Tile Renderer

- Scope: render one 128×128 tile map from a Rust-produced snapshot and measure FPS/frame time.
- Out of Scope: pathfinding, world generation, art pipeline, NPC rendering.
- Dependencies: CHRON-002, CHRON-003.
- Files: Godot renderer/scene/assets, Rust render DTO, benchmark harness.
- Tests: tile-count/content validation and visual/runtime smoke test.
- Benchmark: FPS, CPU/GPU frame time, idle vs snapshot refresh scenarios on M5 16GB.
- DoD: all 16,384 tiles render correctly; measurement method and stable FPS are reported.

### CHRON-012 — Snapshot Prototype

- Scope: versioned simulation snapshot, binary serialization, zstd compression, restore validation and indexes needed by the prototype.
- Out of Scope: final save compatibility or historical replay UI.
- Dependencies: CHRON-004, CHRON-005, CHRON-006, CHRON-009.
- Files: snapshot/storage modules, fixtures, ADR.
- Tests: round trip, corruption/version rejection, ECS-handle exclusion, restored invariants.
- Benchmark: raw/compressed size plus encode/compress/decompress/restore time.
- DoD: restored state matches stable domain state and reportable size/time measurements exist.

### CHRON-013 — SQLite Event Store Prototype

- Scope: SQLite schema, WAL mode, transactions/batching, event append/query prototype, checkpoint-safe behavior.
- Out of Scope: final retention policy, all archive tables, `.world` packaging.
- Dependencies: CHRON-009.
- Files: `crates/sim-storage/**`, migrations/schema, fixtures, ADR.
- Tests: append/query ordering, causal references, rollback, reopen, WAL/checkpoint, integrity checks.
- Benchmark: events/s and DB growth for documented batch sizes and durability settings.
- DoD: structured events survive reopen, integrity passes, and exact SQLite settings/results are recorded.

### CHRON-014 — Architecture Spike Report

- Scope: consolidate verified Phase 0 results and recommendations in `docs/reports/ARCHITECTURE_SPIKE_V1.md`.
- Out of Scope: Phase 1 implementation or invented/unmeasured conclusions.
- Dependencies: every Phase 0 implementation, benchmark, CI, and governance task.
- Files: final report and referenced raw result artifacts only.
- Tests: links/commands/results audited; required fields present; clean CI rerun.
- Benchmark: final reference-machine suite, including headless/rendered comparison.
- DoD: contains every user-required metric, known risks, both technology recommendations, and product-owner decisions; Phase 1 remains blocked pending confirmation.

### CHRON-015 — Test, Lint, Benchmark, and CI

- Scope: repeatable local commands and GitHub Actions for formatting, lint, unit/integration tests, Godot validation where available, and benchmark smoke checks without asserting noisy performance in shared CI.
- Out of Scope: deployment, release signing, or replacing M5 reference benchmarks with CI numbers.
- Dependencies: CHRON-001; expands as later tasks land.
- Files: workflow/config/tool scripts and CI documentation.
- Tests: workflow syntax, local-equivalent commands, required checks fail on deliberate fixture violations where practical.
- Benchmark: compile/run smoke only in CI; M5 performance gates remain local and recorded.
- DoD: clean checkout runs documented checks; no test is skipped or weakened; artifacts preserve benchmark context.

### CHRON-016 — Event Throughput Benchmark

- Scope: generate, serialize, route, and optionally persist structured events under controlled workloads.
- Out of Scope: NLG strings, final gameplay event mix, hiding storage cost inside an in-memory number.
- Dependencies: CHRON-009; storage scenario additionally depends on CHRON-013.
- Files: event benchmark harness and raw result artifacts.
- Tests: generated-event validity and exact event-count assertions.
- Benchmark: in-memory events/s, serialized bytes/event, SQLite events/s reported separately.
- DoD: repeatable M5 16GB throughput and variance are recorded with batching/durability configuration.

## Expected ADRs

1. ADR-0001 Rust workspace boundaries and dependency direction.
2. ADR-0002 persistent `EntityId` representation, allocation, and serialization.
3. ADR-0003 simulation time representation and serialization.
4. ADR-0004 Scheduler ordering, cancellation, and deterministic tie policy.
5. ADR-0005 `bevy_ecs` spike decision after measured alternatives/fit.
6. ADR-0006 structured event schema and causality references.
7. ADR-0007 Godot GDExtension boundary, ownership, batching, and render DTO.
8. ADR-0008 SQLite schema/WAL/durability and event append contract.
9. ADR-0009 snapshot format, schema versions, compression, and restore contract.
10. ADR-0010 developer metrics ownership and sampling boundary.

Any later public cross-module contract change requires a new ADR or a superseding ADR. A conflict with `MASTER_SPEC.md` requires a Change Proposal and stops only the conflicting implementation.
