# Phase 1 — Micro World Kernel Plan

> Current execution/status entry: [CURRENT_PROGRESS](CURRENT_PROGRESS.md).
> CHRON-027–031 are implemented and locally verified (027–029 via the
> [V2 repair](reports/P1_KERNEL_REPAIR_V2.md); 030 via
> [its report](reports/CHRON-030_SIMULATION_WORKER.md); 031 via
> [its report](reports/CHRON-031_GODOT_MICRO_WORLD.md)). Historical
> planning/proposed language below does not override per-task approvals or
> authorize CHRON-032+.

> Approval clarification (product owner, 2026-08-30): an explicit instruction
> to follow/execute an identified plan accepts that plan's stated decisions
> and implementation steps without repeated per-Task or dispatch approval.
> A planning/review request alone does not. This clarification supersedes
> routine reconfirmation wording below; see `AGENTS.md`. The currently
> accepted remediation plan does not include CHRON-027+ implementation.

Current execution surface for the remaining work:
[P1-REMAINING / 2026-08-30-r1](PHASE_1_REMAINING_EXECUTION.md), together with
CHRON-027–036. It specifies recommended semantics, supporting files/tools,
commands, ownership and the terminal boundary. It remains **Proposed**, not
implementation approval. All future plans use [Execution Contract](EXECUTION_CONTRACT.md).

- Phase: **1 — Micro World Kernel**
- Owner: Palimpsest Phase 1 planning document author
- Reference machine: Apple M5, 10 cores, 16 GiB unified memory — same as Phase 0
- Rust: 1.98.0 stable (workspace MSRV `1.95`), Edition 2024, resolver 3
- Baseline authority: `MASTER_SPEC.md` (read-only; SHA-256
  `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`)
- Confirmed spike: `docs/reports/ARCHITECTURE_SPIKE_V1.md`, product-owner confirmed 2026-08-29

`MASTER_SPEC.md` remains the read-only, highest-authority product specification.
This plan implements a **bounded subset** of it and may not override it, `AGENTS.md`,
`docs/ARCHITECTURE.md`, `docs/PERFORMANCE.md`, or any accepted ADR. Where a
requested scope conflicts with the Master Spec, create a Change Proposal
(`docs/proposals/CP-XXXX.md`) and stop the conflicting implementation.
Other contract changes require the relevant ADR/plan update before code; ordinary
in-scope documentation synchronization does not require a CP.

---

## 1. Purpose and Phase Boundary

Phase 1 builds the smallest headless kernel that proves the Master Spec's core
causal loop for a **single Local world**: world grid, terrain, a single
128×128 Local Tile, Person Entity, Basic Movement, Time, Needs, and Basic
Utility AI, and it validates that loop headlessly for **100 NPCs
moving / eating / sleeping / working across 10 continuous years without
crashing**.

Phase 1 does **not** claim to prove a 200-year world, full MVP closure, or the
later scale gates (1K / 3K / 5K / 10K). It produces the deterministic kernel,
its render-oriented DTO + Godot micro-world presentation, its 10-year chaos
gate, representative scale benchmarks, deterministic regression/CI, and a
validation report.

### 1.1 What Phase 1 authorizes

- Phase 1 **planning**, per-`docs/tasks/CHRON-0NN.md` **specifications**, and
  **ADR work** are authorized immediately.
- Phase 1 implementation requires approval of a Task **or an identified execution
  plan containing it**. An instruction to execute that plan accepts its stated
  recommendations and steps once. Continue verified, dependency-ready Tasks
  without repeated approval. Drafting/reviewing a plan alone does not authorize code.

---

## 2. Hard Out of Scope (Phase 1)

These are **not** in Phase 1 scope and must never drift in "because it is
easy". A request to add one requires a scope decision; a CP is required for an
actual Master Spec conflict, not for every ordinary plan clarification.

- Phase 2+ systems: war, politics, religion, magic, machinery/technology,
  disease ecology, open markets, crimes/laws.
- Historians, historiography, NLG, local/remote LLMs, LLM NPC agents, Rule Editor, God Actions,
  narrative text of any kind.
- Web client / WASM, Lua/Rhai modding, multiplayer.
- More than one 128×128 Local Tile in the world; multiple Regions; multi-chunk
  Town/City layouts.
- Population Simulation for wildlife/plants/insects; non-sapient ecology.
- Multi-dimensional relationships, Memory, Knowledge/Belief/Claim/Rumor/Document
  separation, Family, Birth/Aging/Death, Skills/Professions, Inventory,
  Economy, Construction, Organizations.
- Combat of any kind; injury/body-part models.
- Event Store retention policy, Historical Replay, `.world` packaging,
  archive/archive-history UI.
- Real SimTime calendar lore; unit-conversion to "days/years" lore beyond the
  minimal notion needed to run a "year".
- `bevy_ecs` runtime-handle persistence — never allowed, unchanged from ADR-0002.
- Any relaxation of memory or performance budgets (see Sections 12 and 13).
- Removing, skipping, or weakening any test to make checks pass.

Future phases are explicitly deferred, not cancelled.

---

## 3. Architecture Invariants (must hold in every Task)

Derived from `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, and ADR-0001/2/3/4/5/6/7/8/9.

1. **Simulation Core is authoritative and headless.** The Rust simulation runs
   fully without Godot. Godot owns presentation, rendering, input, UI only.
2. **Godot Scene Tree is never simulation truth.** Godot reads immutable
   Render Snapshots; it cannot mutate Simulation Core state.
3. **Persistent identity is a stable domain `EntityId`** (non-zero monotonic
   `u64`, never recycled). Runtime ECS handles are never persisted and never
   cross the bridge/storage boundary (ADR-0002).
4. **Time is `SimInstant` integer seconds**, monotonic `SimClock`, independent
   of wall-clock, frames, and execution speed (ADR-0003).
5. **Scheduling is due-time then FIFO**, deterministic, no internal callback or
   entity scan; queue health observable (ADR-0004).
6. **Structured event truth**, never `Vec<String>`; prose/beliefs/claims remain
   separate (ADR-0006).
7. **Simulation crates forbid unsafe code.** `godot-bridge` has only the
   `ExtensionLibrary` marker exception (ADR-0007). The separate outward-only
   benchmark tool has the accepted private native measurement/probe exceptions
   in ADR-0020; it is not a simulation dependency.
8. **Storage persists stable domain reps only**, never ECS handles or heap
   internals; snapshots are versioned bincode+zstd (ADR-0008/0009).
9. **Dependency direction is inward** toward domain primitives; `sim-core`
   composes them, while outer adapters (`headless-runner`, `godot-bridge`) stay
   at the edge (ADR-0001/0017).
10. **LLM is optional and never decides simulation truth.** Level 0 (Utility AI)
    always exists (MASTER_SPEC §54).
11. **Identity, causality, history fidelity, tests, and determinism are
    preserved during any optimization** (docs/PERFORMANCE.md).
12. **Every important result has a causal, explainable utility source.** No
    `random_action()` as the behavior substrate; randomness is only a scoring
    perturbation (MASTER_SPEC §§2.4, 14).

---

## 4. Fixed Seed and Determinism Rules

These are **non-negotiable** and apply to every simulation task. The kernel must
be **bit-for-bit reproducible for a given (world seed, fixed input sequence)**.

1. Every world/run carries a single `u64` **world seed** in the serializable
   world config. Phase 1 does not claim production snapshot/save support.
2. All randomness flows through an injected, seed-derived PRNG (e.g. a split/`xorshift`,
   ChaCha, or `StdRng` `SeedableRng`) inside Simulation Core. No global `rand`,
   no `thread_rng`, no time-based seeding inside the simulation.
3. **No wall-clock, no frame count, no machine/OS dependency** may influence
   simulation truth. Only `SimInstant` and the seeded PRNG drive simulation.
4. Determinism must be independent of **incidental collection iteration order**.
   Semantically different command order remains part of the explicit input.
   Collections that affect truth use stable ordering (`BTreeMap`, sorted keys,
   explicit entity-id ordering), never unordered `HashMap` iteration that leaks
   into utility or event ordering.
5. Scheduler order is due-time then FIFO (ADR-0004); equal-time work order is
   deterministic by insertion or a stable tie-break.
6. Pathfinding keeps CHRON-024's actual `(f, h, row-major coordinate)` ordering
   and fixed neighbours; it has no EntityId tie-break. No arbitrary heap/queue
   order dependence is allowed.
7. A fixed seed plus fixed config and input-command sequence yields a fixed world,
   population, schedule, decision traces, and event sequence. Any divergence is
   a determinism **regression**, not a flake.
8. Determinism must hold in both **headless and rendered** modes; rendering must
   never feed back into simulation truth (spike report §Headless/Rendered).
9. If a Phase 0 prototype snapshot is used for diagnostics, it must not be
   presented as a supported save or compatibility guarantee (ADR-0016).

---

## 5. Task DAG

```text
018 Workspace Boundaries
 ├─> 019 World Coordinates / Local Grid
 │    ├─> 020 Terrain / World Generation
 │    │    ├─> 023 Activity Sites
 │    │    ├─> 024 Deterministic Pathfinding
 │    │    └─> 031 Godot Presentation
 │    └─> 021 Person Runtime Model
 │         └─> 022 Needs Model
 │
 └─────────────────────────────────────────────┐
                                               │
021 + 022 + 023 + 024 ─> 025 Action/Trace ─> 026 Utility
023 + 024 + 025 + 026 ─> 027 Action Execution
021 + 022 + 027 ───────> 028 Kernel Orchestration
019 + 020 + 021 + 028 ─> 029 Render Snapshot DTO
028 + 029 ─────────────> 030 Simulation Worker / Commands
020 + 029 + 030 ───────> 031 Godot Presentation
028 ───────────────────> 032 10-Year Chaos Runner
028 + 029 + 030 + 031 + 032 ─> 033 Scale Benchmarks
032 + 033 ─────────────> 034 Regression / CI
028 + 033 + 034 ───────> 035 Retire Phase 0 Workload
031 + 032 + 033 + 034 + 035 ─> 036 Validation Report
```

### 5.1 Dependency Matrix

| Task | Depends on | Produces |
| --- | --- | --- |
| 018 Workspace Boundaries | — (established by Phase 0 ADR-0001) | Phase-1 workspace conventions & new crate contracts |
| 019 World Coordinates / Local Grid | 018 | single-Local `WorldGrid`, `LocalGrid`, `LocalCoord` |
| 020 Terrain / World Generation | 019 | deterministic 128×128 terrain |
| 021 Person Runtime Model | 018, 019 | Person identity/location + bevy_ecs runtime mapping |
| 022 Needs Model | 021 | bounded Hunger/Fatigue domain values + Person component integration |
| 023 Activity Sites | 019, 020 | static Meal/Rest/Work value records on the local tile |
| 024 Deterministic Pathfinding | 019, 020 | deterministic grid pathfinding |
| 025 Action & Decision Trace Contracts | 021, 022, 023, 024 | explainable decision-trace contract + structured traces |
| 026 Utility Scoring / Selection | 022, 023, 024, 025 | utility scoring + selection |
| 027 Action Execution State Machine | 023, 024, 025, 026 | action execution states (move/eat/sleep/work) |
| 028 Scheduler / Kernel Orchestration | 021, 022, 027 | composed headless kernel + SimClock / Scheduler wiring |
| 029 Render Snapshot DTO | 019, 020, 021, 028 | immutable render-oriented snapshot DTO |
| 030 Simulation Worker / Command Bridge | 028, 029 | worker + command bridge (not Godot main thread) |
| 031 Godot Micro-World Presentation | 020, 029, 030 | Godot rendering + Person display of the micro-world |
| 032 Headless 10-Year Chaos Runner | 028 | 10-year chaos-run harness + invariants |
| 033 Representative Scale Benchmarks | 028, 029, 030, 031, 032 | representative pathfinding / utility / kernel / memory / bridge benches |
| 034 Deterministic Regression / CI | 032, 033 | regression suite + CI gates |
| 035 Retire Phase 0 Spike Workload | 028, 033, 034 | ADR-0010 retirement; remove `run_spike_workload` |
| 036 Phase 1 Validation Report | 031, 032, 033, 034, 035 | validation report |

### 5.2 Parallel Waves

Waves identify dependency-ready candidates, **not proof of file independence**.
Parent verifies disjoint file ownership before any parallel dispatch (Section 15).
Shared manifests, exports, fixtures and ADRs stay parent-owned and serialized.

- **Wave 1** — `018`
- **Wave 2** — `019`
- **Wave 3** — `020`, `021` (parallel only in isolated worktrees; both may consume `sim-world` public types but may not edit the same files)
- **Wave 4** — `022`, `023`, `024` (parallel only with disjoint implementation files)
- **Wave 5** — `025`
- **Wave 6** — `026`
- **Wave 7** — `027`
- **Wave 8** — `028`
- **Wave 9** — `029`, `032` (parallel; kernel complete, disjoint DTO/runner files)
- **Wave 10** — `030`
- **Wave 11** — `031`
- **Wave 12** — `033`
- **Wave 13** — `034`
- **Wave 14** — `035`
- **Wave 15** — `036`

---

## 6. Global Definition of Done (all Tasks complete)

Quoted against `MASTER_SPEC.md` §73–76 and the confirmed spike. Phase 1 is done
when **all** of the following are true:

1. A single 128×128 Local Tile renders from a Rust render snapshot in Godot at
   **60 FPS (min/mean/p95)** under the Phase 1 micro-world workload, and achieves
   **1 draw call** for the tile map (spike report §Tile rendering remains the
   target).
2. The headless kernel runs a deterministic **10-year, 100-NPC** scenario
   (100 persons moving / eating / sleeping / working) that **completes without
   crash, NaN, infinite loop, dangling Entity reference, or unbounded memory
   growth**, satisfying the Master Spec Chaos Simulation Test (§76).
3. World truth remains authoritative in Rust; Godot contains only presentation
   state. No ECS handle crosses the bridge/storage boundary (ADR-0002/0007).
4. Given the fixed world seed, the headless run is **bit-for-bit reproducible**:
   identical decision traces and event sequence across repeated runs and across
   headless vs. rendered execution.
5. Utility AI is **explainable**: every selected action yields an audit-able
   decision trace (inputs → per-candidate scores → selection) with no
   `random_action()` as the behavioral substrate (§14).
6. High-level action outcomes use structured in-memory events (not
   `Vec<String>`) with stable `EntityId` references. Per-decision traces remain
   bounded runtime diagnostics; Phase 1 does not claim Event Store retention.
7. Memory budgets retained (not relaxed): MVP Core + Client target **< 3 GB RSS**,
   10K target **< 5 GB** (docs/PERFORMANCE.md; spike §M5 Memory Budget). Phase 1
   reports its actual RSS/overhead without compromising the caps.
8. Representative benches for pathfinding, movement, utility, kernel, and memory
   record repeatable M5 16GB results with warm-up, samples, profile, and
   variance (docs/PERFORMANCE.md measurement rules).
9. `CHRON-035` retires the Phase 0 `sim-core::run_spike_workload` shared workload
   and supersedes ADR-0010 with a real path; no Phase 0 mode-probe remains in
   the kernel.
10. Final local gates pass: `rustfmt`, Clippy with warnings denied, all workspace
    tests/targets/features, MSRV 1.95, and the documented bench/Godot smoke.
11. `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md` exists, is auditable, and
    records commands actually run, benchmark results, known limitations, and
    blockers (AGENTS.md task contract).
12. No task left a regression suite weaker than it started; no test was deleted,
    skipped, or disabled to make checks pass.

Implementation requires explicit execution approval of the identified Task or
plan; that approval is recorded once. The current documentation clarification
does not start CHRON-027+.

---

## 7. Task Definitions

Each Task's detailed spec lives in `docs/tasks/CHRON-0NN.md` (Phase 0 layout).
The summaries below are a roadmap. For 018–026, completed Task reports and
accepted ADRs describe the actual implementation. For 027–036, the detailed
Task plus P1-REMAINING form the execution contract; a summary's N/A or placeholder
does not remove a detailed Task's required benchmark or error contract.

### 018 — Workspace Boundaries

- **Purpose:** Make Phase 1's intended crate boundaries concrete and keep
  Simulation Core headless; add only the crates Phase 1 needs.
- Scope: add only `sim-world` and `sim-ai` under ADR-0017; reaffirm dependency rules.
- Out of Scope: any gameplay system.
- Dependencies: ADR-0001 accepted; Phase 0 workspace.
- Files: root `Cargo.toml`, lockfile if mechanically changed, the two new crate
  manifests/source skeletons, Task report, and ADR-0017 status.
- API Contract: no new domain API; only crate-level boundaries.
- Tests: workspace resolves; lint/test baseline; headless-only compilation.
- Benchmark: N/A.
- DoD: workspace builds headless; no Godot/LLM dependency in sim crates; boundary
  matches ADR-0001/0007.

### 019 — World Coordinates / Local Grid

- **Purpose:** Define a minimal `WorldGrid` containing one canonical 128×128
  `LocalGrid` and its checked local coordinate system.
- Scope: `WorldGrid`, `LocalGrid<T>`, `LocalCoord`, row-major indexing and bounds.
- Out of Scope: multi-chunk, multiple regions, terrain, pathfinding.
- Dependencies: 018.
- Files: spatial module, tests, and ADR-0012 status/decision evidence.
- API Contract: stable coordinate type with bounds-checked indexing.
- Tests: bounds, wrap/index math, ordering, serialization round trip.
- Benchmark: N/A.
- DoD: coordinates are deterministic and persistence-safe; Local is exactly 128×128.

### 020 — Terrain / World Generation

- **Purpose:** Generate a deterministic 128×128 terrain from the world seed.
- Scope: seeded generation of a minimal closed `TerrainKind` set and
  walkability, with deterministic map values.
- Out of Scope: living ecology, weather, geography evolution, construction.
- Dependencies: 019.
- Files: terrain module, generator, tests, fixtures.
- API Contract: `generate_terrain(seed) -> LocalTerrain` deterministic.
- Tests: same seed → identical map; bounds; land/water counts; walkability
  invariants; seed-differs → (likely) differs.
- Benchmark: generation time at 128×128.
- DoD: identical seed yields byte-identical terrain; generation is reproducible.

### 021 — Person Runtime Model

- **Purpose:** Model a sapient Person with stable `EntityId` and a runtime-ECS
  handle mapping.
- Scope: Person marker, stable identity, location, runtime mapping, and spawn in
  the single Local. Needs and CurrentAction are attached by later Tasks.
- Out of Scope: body, needs, actions, AI, relations, skills, family, memory.
- Dependencies: 018, 019.
- Files: person module, runtime mapping, tests.
- API Contract: stable `EntityId` from ADR-0002; runtime handle not exposed across
  persistence/bridge.
- Tests: id uniqueness, handle mapping, spawn placement, no handle in DTO bounds.
- Benchmark: spawn cost (relevant at scale later).
- DoD: persons exist with stable identity and runtime handles, headless.

### 022 — Needs Model

- **Purpose:** Give each Person measurable Needs that drive utility.
- Scope: exactly hunger and fatigue components, update over elapsed SimDuration,
  clamped ranges, persistence-ready values.
- Out of Scope: Relationship/memory effects, skills, disease.
- Dependencies: 021.
- Files: needs module, tests.
- API Contract: needs update is a pure function of (person, dt); values bounded.
- Tests: monotonic-clamped update, zero/overflow bounds, determinism.
- Benchmark: N/A.
- DoD: needs are deterministic, bounded, and persistence-safe.

### 023 — Activity Sites

- **Purpose:** Provide concrete places on the Local tile for actions (eat / sleep / work).
- Scope: static Meal/Rest/Work affordance values, tile placements, availability,
  bounded work observation counter, deterministic nearest-site selection.
- Out of Scope: buildings, construction, ownership, economy.
- Dependencies: 019, 020.
- Files: activity-site module, placements, tests.
- API Contract: sites are immutable values addressed by `LocalCoord` and kind;
  they are not persistent buildings or inventory-owning entities.
- Tests: placement validity, availability, deterministic selection, serialization.
- Benchmark: N/A.
- DoD: sites are stable, placed deterministically, and queryable by utility.

### 024 — Deterministic Pathfinding

- **Purpose:** Route a Person across walkable tiles deterministically.
- Scope: grid A*/BFS on the 128×128 Local, deterministic tie-break, obstruction
  handling, path costs.
- Out of Scope: multi-tile regions, dynamic re-planning, agents/movement crowds.
- Dependencies: 019, 020.
- Files: pathfinding module, tests, benches.
- API Contract: `find_path(from, to, terrain) -> Vec<LocalCoord>` deterministic.
- Tests: valid path, unreachable handling, tie determinism, bounded runtime.
- Benchmark: path time, avg path length, worst-case path (033 repeats at scale).
- DoD: same grid+query yields identical path; no allocation explosion; no hang.

### 025 — Action & Decision Trace Contracts

- **Purpose:** Define the explainable decision-trace contract and structured trace
  record so Utility AI is audit-able.
- Scope: bounded runtime `DecisionTrace`/`ActionCandidate` values for Developer
  Metrics and tests; no durable per-decision Event Store append.
- Out of Scope: the scoring logic itself (026), execution (027).
- Dependencies: 021, 022, 023, 024.
- Files: decision-trace module, tests, and ADR-0014 status/decision evidence.
- API Contract: trace includes inputs → factor contributions → selection with
  `EntityId`/`SimInstant`; it is read-only diagnostic state.
- Tests: trace completeness, bounded size, deterministic ordering, serde round trip if exposed.
- Benchmark: N/A (trace weight tracked in 033).
- DoD: every decision is auditable from a bounded runtime trace; high-level
  outcomes use structured in-memory events, with no durable per-decision append.

### 026 — Utility Scoring / Selection

- **Purpose:** Score candidate actions from Perception + Needs and select the top
  with a deterministic, explainable rule plus bounded score noise.
- Scope: utility function per goal (eat/sleep/work/move), weighting, deterministic
  tie-break, bounded randomized perturbation.
- Out of Scope: GOAP/planning, long-term goals beyond the basic set, real LLM.
- Dependencies: 022, 023, 024, 025.
- Files: utility module, selections, tests, benches.
- API Contract: selection returns the chosen action + its decision trace.
- Tests: deterministic selection, tie-break, invariance to hash order, boundedness.
- Benchmark: utility evaluation per thousand persons (033 at scale).
- DoD: explainable selection; no `random_action()` substrate; deterministic under seed.

### 027 — Action Execution State Machine

> Implemented 2026-08-30 under P1-REMAINING / 2026-08-30-r1; contract ADR-0021;
> evidence `docs/reports/CHRON-027_ACTION_STATE_MACHINE.md`.

- **Purpose:** Run a selected action through concrete states (moving → arriving →
  performing → done) as a scheduled state machine.
- Scope: action states for move/eat/sleep/work, Needs satisfaction on completion,
  event emission, interruption/invalid-params handling.
- Out of Scope: skills, production chains, combat, multi-agent interactions.
- Dependencies: 023, 024, 025, 026.
- Files: action module, state machine, tests.
- API Contract: actions are scheduled work with deterministic transitions;
  high-level outcomes may emit bounded in-memory structured events.
- Tests: happy path, interruption, invalid targets, state invariant, no infinite loop.
- Benchmark: 100/1,000-person transitions, peak RSS and queue growth (027 + P1-REMAINING).
- DoD: actions complete deterministically, emit structured events, never loop forever;
  the ADR-0018 actual decide/move/execute/needs loop passes.

### 028 — Scheduler / Kernel Orchestration

- **Status — implemented 2026-08-31 (ADR-0022), repaired 2026-08-31 (ADR-0024); see `docs/reports/CHRON-028_KERNEL.md` and `docs/reports/P1_KERNEL_REPAIR_V1.md`.**
- **Purpose:** Compose clock, scheduler, persons, needs, sites, pathfinding, and
  actions into one headless world kernel.
- Scope: `SimClock`/`Scheduler` wiring (ADR-0003/0004), systems tick on demand
  (not per-second scan), world struct, deterministic step, event/snapshot hooks.
- Out of Scope: storage integration, render worker, any gameplay beyond micro-world.
- Dependencies: 021, 022, 027.
- Files: kernel module, wiring, integration tests.
- API Contract: `advance_to(target, work_budget)` deterministic; systems fire only when due.
- Tests: determinism across runs, due-time FIFO, no cross-tick leak, invariant checks.
- Benchmark: kernel throughput and queue health.
- DoD: the kernel advances deterministically, honors scheduling, exposes metrics.

### 029 — Render Snapshot DTO

- **Status — implemented 2026-08-31 (ADR-0023, schema 2), repaired 2026-08-31 (ADR-0024); see `docs/reports/CHRON-029_RENDER_SNAPSHOT.md` and `docs/reports/P1_KERNEL_REPAIR_V1.md`.**
- **Purpose:** Publish an immutable, render-oriented snapshot for Godot from Rust.
- Scope: DTO for terrain + Person positions + minimal micro-world state, batching,
  no simulation mutation crossing the boundary.
- Out of Scope: game logic in Godot, asynchronous worker (030).
- Dependencies: 019, 020, 021, 028.
- Files: render DTO module, bridge-facing types, tests.
- API Contract: snapshot is immutable and presentation-only; no ECS handle.
- Tests: DTO completeness, entity-id references stable, no mutation path, determinism.
- Benchmark: snapshot build/transfer cost (batched).
- DoD: Godot can reconstruct presentation purely from the snapshot.

### 030 — Simulation Worker / Command Bridge

- **Status — implemented and locally verified 2026-08-31 (ADR-0015 Phase 1
  supplement); see `docs/reports/CHRON-030_SIMULATION_WORKER.md`.**
- **Purpose:** Run simulation off Godot's main thread and publish immutable,
  batched snapshots, plus accept commands.
- Scope: worker thread driving the kernel, batched snapshot publication, command
  channel (start/pause/step/speed), no separate process (spike decision #6).
- Out of Scope: threading model beyond a worker; persistence; real-time UI.
- Dependencies: 028, 029.
- Files: worker module, bridge command types, tests, and ADR-0015 conformance evidence.
- API Contract: worker owns simulation; Godot requests/publishes snapshots; no
  main-thread simulation truth.
- Tests: no frame-stall feedback, correct batching, determinism preserved, clean shutdown.
- Benchmark: snapshot publish rate, frame-stall measurement.
- DoD: simulation runs off main thread; Godot sees immutable snapshots; no
  main-thread simulation tick is introduced. Frame cost/latency is measured
  by 031, not asserted to be universally zero.

### 031 — Godot Micro-World Presentation

- **Status — implemented and locally verified 2026-08-31 (ADR-0026); see
  `docs/reports/CHRON-031_GODOT_MICRO_WORLD.md`.**
- **Purpose:** Render the deterministic micro-world from snapshots and show Persons.
- Scope: tile rendering from DTO, person/activity-site markers, camera, 60 FPS target.
- Out of Scope: full UI panels, inspector, archive; pause/resume/speed/step are in scope.
- Dependencies: 020, 029, 030.
- Files: `apps/macos-godot/**` presentation, scenes, scripts.
- Tests: visual/runtime smoke, FPS measurement over N frames, snapshot-driven render.
- Benchmark: FPS min/mean/p95, 1 draw call, GPU/CPU frame time, idle vs refresh.
- DoD: 60 FPS target met; presentation driven purely by snapshots.

### 032 — Headless 10-Year Chaos Runner

- **Purpose:** Run 100 NPCs moving/eating/sleeping/working for 10 simulated years
  headlessly and assert chaos invariants.
- Scope: scenario builder (seed + 100 persons + sites), long-run loop, invariant
  gates (no NaN, no infinite loop, no dangling `EntityId`, no instant extinction,
  no unbounded resource/memory), completion report.
- Out of Scope: ecology, economy, family growth, RL systems; 200-year claim.
- Dependencies: 028.
- Files: chaos-runner binary, scenario fixtures, invariants, report artifacts.
- API Contract: fixed seed → deterministic 10-year run; machine-readable result.
- Tests: 10-year run completes; invariants hold every checkpoint; reproducible.
- Benchmark: wall time / RSS for the 10-year run.
- DoD: 10 years of 100 NPCs completes without crash, NaN, hang, dangling
  reference, or memory blowup; reproducible from seed.
- **Status — implemented 2026-08-31 (ADR-0027); see `docs/reports/CHRON-032_CHAOS_10YEAR.md`.**

### 033 — Representative Scale Benchmarks

- **Purpose:** Measure the Phase 1 kernel at representative (and staged repeat)
  scale on the M5 16GB machine.
- Scope: benches for pathfinding, movement, utility, kernel throughput, memory
  (RSS), following the docs/PERFORMANCE.md measurement rules; report artifacts.
- Out of Scope: claiming 1K/3K/5K/10K full-NPC gameplay support; noisy CI assertions.
- Dependencies: 028, 029, 030, 031, 032.
- Files: bench harnesses, fixtures, `docs/reports/CHRON-033_*.md`.
- API Contract: benches use production paths; correctness assertions enabled.
- Tests: bench correctness + exact count assertions.
- Benchmark: repeatable M5 data with warm-up, samples, distribution, profile.
- DoD: reproducible scale data recorded; no budget relaxation; methods documented.

### 034 — Deterministic Regression / CI

- **Purpose:** Lock determinism and correct behavior into local + hosted CI.
- Scope: regression suite asserting identical output for a fixed seed, local gate
  script, GitHub Actions jobs (Ubuntu Rust quality/smoke + arm64 macOS Godot
  integration), no noisy bench assertions in shared CI.
- Out of Scope: deployment, signing, release; relaxing/weakening tests.
- Dependencies: 032, 033.
- Files: workflow/config/tool scripts, regression fixtures, CI docs.
- API Contract: same seed → same hash; CI is the enforcement gate.
- Tests: workflow syntax, local-equivalent commands, determinism fixtures fail on
  violation.
- Benchmark: compile/run smoke only in CI; M5 gates stay local.
- DoD: clean checkout passes checks; determinism regression is caught; no test
  weakened.

### 035 — Retire Phase 0 Spike Workload

- **Purpose:** Remove the Phase 0 `sim-core::run_spike_workload` shared workload
  and supersede ADR-0010 now that a real kernel + Godot path exists.
- Scope: delete/replace spike workload & mode-probe, re-point headless and Godot
  path to the real kernel, supersede/close ADR-0010, drop spike-only deps.
- Out of Scope: any new gameplay; keeping spike code as "useful" residue.
- Dependencies: 028, 033, 034.
- Files: `sim-core` spike removal, runner/bridge rewiring, ADR supersession, tests.
- API Contract: no public spike API remains; headless & Godot use the kernel.
- Tests: full suite still green; headless/rendered path both use kernel.
- Benchmark: confirm no spike-only perf deviation.
- DoD: ADR-0010 superseded, spike workload removed, tests pass, nothing regressed.

### 036 — Phase 1 Validation Report

- **Purpose:** Consolidate verified Phase 1 results into an auditable report.
- Scope: write `docs/reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md` with change summary,
  commands run, benchmark results, known limitations, blockers, and DoD evidence.
- Out of Scope: Phase 2 entry, invented/unmeasured claims, implementing new work.
- Dependencies: 031, 032, 033, 034, 035.
- Files: report plus referenced raw artifacts only.
- Tests: links/commands/results audited; required fields present; clean CI re-run.
- Benchmark: consolidate the final M5 reference suite; reruns belong to the
  originating measurement Task, not a new workload in this report Task.
- DoD: report evidences every global DoD item (Section 6) and records known
  limitations/blockers; stable Phase 1 status is recorded.

---

## 8. Schema, Public API, and ADR Gates

- Any change to a **cross-module public API**, database/schema, identity, ECS,
  serialization, Godot bridge, AI, history retention, NLG, or Rule IR contract
  requires an **ADR** before implementation (AGENTS.md Change Governance).
- Accepted Phase 1/measurement decisions now include ADR-0011–0020. Remaining
  Tasks record their new public execution/kernel/DTO/worker contracts before
  implementation; this is planned work, not an extra owner-approval ceremony.
- Conflicts with `MASTER_SPEC.md` produce a `docs/proposals/CP-XXXX.md` (template
  exists at `docs/proposals/TEMPLATE.md`) and **stop** only the conflicting
  implementation.
- Do not delete, skip, weaken, or disable tests to make checks pass.
- Do not relax performance budgets without product-owner approval.
- A superseding ADR (e.g., replacing ADR-0010 in 035) must state what it
  supersedes and why.

---

## 9. Performance Gates (Phase 1)

Measured on the M5 16GB reference machine per `docs/PERFORMANCE.md`. Gates must
be **retained**, never relaxed, without product-owner approval.

- **60 FPS (min/mean/p95) tile rendering** for the single 128×128 Local with
  Person/site markers under the Phase 1 workload; **1 draw call** for the base
  tile map (spike target).
- **Kernel determinism / throughput**: 100 NPCs across 10 years headless must
  complete in a bounded, recorded time with no hang and no infinite loop; report
  the number and time.
- **Memory caps retained**: MVP Core + Client target **< 3 GB RSS**; 10K target
  **< 5 GB**; overall env with optional Tiny LLM **< 7 GB**. Phase 1 reports its
  RSS/overhead; it cannot reduce or declare these caps fully validated.
- **Measurement rules enforced**: release builds, warm-up, sample count, exact
  command, dependency versions, median + limitations; correctness assertions
  enabled; never treat one noisy CI run as a gate.
- **Stage repeat**: measure representative workloads at 100 / 1K / 3K / 5K /
  10K where applicable. Only 100 is a Phase 1 gameplay gate; larger results are
  diagnostics and are not claims that full NPC gameplay is supported at scale.
- Benches required in Phase 1 scope: pathfinding, movement, utility, kernel,
  memory (per MASTER_SPEC §75 subset for this phase).

---

## 10. 10-Year Chaos Gate (Master Spec §76)

`CHRON-032` is the Phase 1 chaos gate. A headless run must assert **all** of:

- all 100 Persons persist because Phase 1 implements no death/removal system;
- resources/needs do not grow unbounded (bounded clamps hold);
- no NaN / invalid numeric state;
- no infinite loop / non-terminating step (bounded per-step and total time);
- no dangling `EntityId` reference (every reference resolves, or is explicitly
  external);
- storage/database is N/A for this gate because Phase 1 does not implement
  production persistence; any optional prototype diagnostic must remain valid;
- long run shows no obvious memory leak (track RSS growth across the run);
- determinism: the same seed reproduces the same 10-year state hash, invariant
  samples, action counts, decision/event sequence; timing and RSS are excluded
  from deterministic equality.

A **failed gate blocks** the Task from being marked complete and blocks
`CHRON-033`/`CHRON-036` evidence.

---

## 11. Stop Conditions

The following prevent acceptance of affected work. Fix ordinary defects within
the approved Task; stopping for a new user decision is reserved for the real
scope/authority blockers in `EXECUTION_CONTRACT.md`:

- A requested Task/change falls outside Phase 1 scope (Section 2) → report the
  scope decision; create a CP if it conflicts with Master Spec. Do not extend
  the phase implicitly.
- A requested change conflicts with `MASTER_SPEC.md` → create a CP and stop the
  conflicting implementation.
- A public cross-module contract needs its planned ADR → write/review it before
  code, then continue. Ask only if the semantics materially depart from the plan.
- The isolated **Godot `--headless --editor --quit` editor-exit crash recurs** in
  normal editor, game, or CI paths (spike §Known Risks #5). Recurrence in a
  normal path is a stop signal; the isolated editor-exit-only case is monitored.
- A **genuine non-monotonic performance or memory regression** beyond documented
  variance → stop and fix before proceeding.
- A determinism regression (same seed, different output) → stop; it is a bug, not
  a flake.
- Neither the Task nor its containing execution plan is approved → no implementation.

---

## 12. Whole-Plan Approval and Internal Readiness

1. Codex prepares an identifiable execution plan and Task contracts, including
   decisions, affected callers, tests, measurement tools, CI and documents.
2. The product owner approves that plan once, or explicitly selects individual
   Tasks. An instruction to follow the identified plan is full acceptance of
   its stated recommendations and execution steps.
3. Parent verifies prerequisites and writes exact public contracts/ADRs before
   code. These are internal readiness steps, not new owner approvals.
4. Implement, test, benchmark and independently review; then continue to the
   next ready Task already in the approval. No evidence, no Done.
5. Stop at the plan's end or for a material unplanned decision/Master conflict/
   genuine unresolvable blocker. Phase 2 requires a new authorization.

---

## 13. Change Governance (unchanged from AGENTS.md)

- Never modify `MASTER_SPEC.md`. Use the Task's implementation and supporting-file
  allowances, including bounded same-module helpers/tests and relevant ADR/docs
  synchronization. Refine exact filenames within that allowance without another
  approval; unrelated edits remain forbidden.
- Master Spec conflicts → CP-XXXX + stop. Other material contract changes → ADR
  and, when not already settled by the plan, a focused decision increment.
- Public API/identity/ECS/serialization/bridge/AI/history/NLG/Rule IR decisions →
  ADR.
- No test deletion/weakening; no budget relaxation without product-owner approval.

---

## 14. GitHub Branch Protection

**Current policy — product-owner decision, 2026-08-30:**
`GabrielMu2006/Palimpsest` must remain **public** on an ongoing basis. This
supersedes all previous requirements to keep it private. Agents must not change
it to private without a new explicit product-owner decision.

`main` must require `rust-quality-and-smoke-benchmarks` and
`godot-macos-integration` in strict mode (branches must be up to date), enforce
the rules for administrators, and prohibit force pushes and branch deletion.
The former recommendation to upgrade GitHub for private-repository protection
is no longer part of the plan. Public visibility does not weaken any CI or
Task-approval requirement.

Historical record, not current policy: the repository began private, was made
public with protection verified on 2026-08-29, and was subsequently made private
again, when the protection API returned HTTP 403. The 2026-08-30 decision settles
the policy as continuously public. This documentation update does not change or
reverify GitHub settings. REM-001 in
`docs/reports/PHASE_1_REVIEW_REMEDIATION_PLAN_V1.md` must verify live public
visibility and the exact `main` protection before claiming enforcement. Any
missing protection is a blocker to report, not a reason to silently change
visibility, lower checks, or substitute a manual policy as equivalent protection.

Live REM-001 read-only check, 2026-08-30 (Asia/Shanghai):
`gh repo view GabrielMu2006/Palimpsest --json nameWithOwner,visibility,url`
returned `PRIVATE`. `gh api repos/GabrielMu2006/Palimpsest/branches/main/protection`
returned HTTP 403 with the public-repository/plan requirement. The first
visibility request timed out; the retry produced this concrete result. No
remote setting was changed. This is a policy mismatch, not a new private
policy or verified protection. Explicit authorization to make the live
repository public and restore the exact approved protections was requested;
REM-001 remained unresolved at that checkpoint.

After the product owner's four-item confirmation on 2026-08-30, REM-001 made
the actual repository PUBLIC. The now-accessible protection endpoint initially
returned 404 (no protection), so the exact approved protections were restored.
Fresh read-back at approximately 12:54 +08:00 confirmed PUBLIC, strict=true,
both required check names (GitHub Actions app ID 15368), enforce_admins=true,
allow_force_pushes=false, and allow_deletions=false. PR #1 remains Draft at
head `e5b0aeb676372a123dd8c27190e94b6a606d498c`; no push/merge or test-status
mutation was used to verify the policy. Detailed commands and limitations
are in the remediation report's execution continuation.

---

## 15. Codex / Optional Agent Execution Protocol

Codex owns design, integration and independent acceptance. Agent use is not a
prerequisite for progress. When the user requests delegation, use the applicable
skill and dispatch only bounded, internally assessed tasks. No dispatch approval
is needed for work already in the approved plan.

### 15.1 Model and routing

- When Luna is requested, use `codex-luna-dispatch` and GPT-5.6 Luna for suitable
  leaf implementations/tests/adapters; unresolved design stays with the parent.
- When OpenCode is explicitly selected, use the user's configured DS provider:
  `deepseek/deepseek-v4-flash-vision-exp`. Do not use `opencode-go/*` and do not
  silently fall back to another external provider/model if it is unavailable.
  Report the routing limitation; Codex may take over in-scope work unless the
  user specifically requires that external route. Do not rewrite user config.
- Route work through the project Skills: start with `palimpsest-task-executor`;
  add `palimpsest-rust-sim` for Rust Core work, `palimpsest-architecture-guard`
  for cross-module/architecture changes, `palimpsest-performance-gate` for
  performance/memory/LOD/benchmark work, `palimpsest-godot-rust` for Godot/bridge
  work, `palimpsest-sim-debug` for long-run simulation anomalies. Do not load
  every Skill for ordinary coding (docs/TOOLING.md routing matrix).

### 15.2 Multi-agent concurrency rule

Multiple agents may run **in parallel only** when:

- their Tasks fall in the same Wave (Section 5.2) AND
- they touch **disjoint files** (no overlap) AND
- their dependencies are already verified complete against the DAG.

Otherwise work is strictly serialized. **No two agents may modify a shared public
interface or a shared module at the same time.**

### 15.3 Branch / worktree isolation

- Preserve the current dirty worktree. Use `codex/` branches when creating new
  ones; isolate concurrent work by disjoint ownership or separate worktrees.
  A sequential approved plan may share a branch with task-separated changes.
- Commit/push/merge only as explicitly included in the execution plan; never
  push unrelated work or treat task completion as automatic merge permission.
- An agent must not update git config, skip hooks, force-push, create empty
  commits, or amend a failed commit.

### 15.4 Test-before-commit

- Write/run tests **before** committing; never commit broken or untested state.
- Run the deterministic gate (same seed → identical output) where applicable.
- Each Task records the **exact commands actually run** and the results in its
  report/DoD (AGENTS.md task contract).

### 15.5 Independent Codex review gate

- Before a Task is accepted, the changed surface is reviewed by code against the
  four lenses: **diff**, **architecture**, **tests**, and **benchmark/CI**.
- The reviewer verifies no scope expansion, no architecture violation, no test
  weakening, no budget relaxation, and (where applicable) the determinism/
  chaos-gate evidence.
- The reviewer is independent of the implementer (a separate agent/pass).

### 15.6 Public API and ADR discipline

- **No multiple agents** may edit a public interface concurrently.
- **Any public API change requires an ADR first** (Section 8), then implementation.
- Do not bypass the ADR step "because it's a small rename".

### 15.7 Failure must not weaken tests

- If a test fails, fix the defect or the code; **never** disable/skip/delete the
  test, weaken its assertions, or gate it behind a flag to make checks pass.
- If a test is genuinely invalid, that is a documented decision requiring
  reviewer + (for product-affecting changes) product-owner approval.

---

## 16. Reference Basis

- `MASTER_SPEC.md` — authoritative product spec (read-only).
- `docs/ARCHITECTURE.md` — Phase 0 architecture baseline (this plan does not
  override it).
- `docs/PERFORMANCE.md` — measurement rules and results index.
- `docs/reports/ARCHITECTURE_SPIKE_V1.md` — confirmed spike; Phase 0 complete.
- `docs/tasks/TEMPLATE.md` + `docs/tasks/CHRON-0NN.md` — Phase 0 task contract.
- Accepted ADRs 0001–0020, with their recorded phase/prototype limits. New
  remaining-task decisions are Proposed in P1-REMAINING until execution approval.
- `AGENTS.md` — repository instructions (Scope/Phase, architecture, governance).
