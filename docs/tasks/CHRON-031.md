# CHRON-031 — Godot Micro World Presentation

> **Status: Implemented and locally verified 2026-08-31 — see
> [report](../reports/CHRON-031_GODOT_MICRO_WORLD.md) and
> [ADR-0026](../adr/ADR-0026-phase-1-godot-presentation-contract.md).**
> Approval of this Task **or its identified execution plan** authorizes its stated steps once.
> Follow [Execution Contract](../EXECUTION_CONTRACT.md) and
> [remaining-plan decisions, supporting files and commands](../PHASE_1_REMAINING_EXECUTION.md).
> Internal design/readiness and agent dispatch do not require repeated owner approval.

## Objective
Present the Phase 1 micro world in the Godot macOS client: render up to 100 persons moving over terrain tiles, provide time controls (pause/speed/step), and show developer metrics, all driven by the Rust Render Snapshot (CHRON-029) and the Simulation worker (CHRON-030), with the Scene Tree kept strictly read-only and a 60 FPS target on the M5 reference machine.

## Context
Phase 0 demonstrated a 128×128 TileMap at a stable 60 FPS and a minimal bridge returning a tiny dictionary. Phase 1 must now render actual moving Persons whose positions and actions come from the authoritative Rust kernel, not from Godot nodes. Per `MASTER_SPEC.md` §8 and ADR-0007, Godot must only ask "where is Person 8127" and draw the returned snapshot; it must never become simulation truth. This Task is the first full view of the micro world in the client and the honest test of the 60 FPS budget for 100 animated persons plus tiles (CHRON-011 proved tiles alone; this adds movement and per-person state).

## Scope
- Consume `RenderSnapshot` (CHRON-029) through the Godot bridge; rebuild tiles and person sprites from the snapshot, never from authoritative mutation.
- Render the terrain/local-tile batch as a TileMap (reuse/refactor the CHRON-011 renderer) and up to 100 persons as simple, readable sprites whose tile positions and current `ActionKind` are read from the snapshot.
- Add a read-only presentation node set: the Scene Tree holds only presentation state, no simulation truth, and no path overwrites kernel values.
- Wire the Simulation worker (CHRON-030) so the client issues pause/speed/step commands and reads the latest complete snapshot; presentation never blocks the kernel mid-tick except through the worker's tick-boundary API.
- Show a developer metrics overlay (CHRON-010) reflecting snapshot metrics (scheduler queue, person count, events/s, TPS/advance info) labeled clearly, never inventing client-owned simulation values.
- Provide minimal time controls: Pause, Resume, a bounded set of Speed multipliers, and N-step. These issue commands to the worker; they do not mutate world state.
- Target 60 FPS on the M5 16GB reference machine while rendering 100 moving persons plus the tile map, and document the measured FPS.

## Out of Scope
- Any authoritative simulation mutation from Godot; Scene Tree state is never simulation truth.
- Movement/actions/utility AI/pathfinding content itself (all Rust-side, CHRON-022/CHRON-025/CHRON-027) — Godot only draws.
- Resources, economy, production, needs-satisfaction content, inventory, building.
- Final art, animation rigs, spritesheets beyond readable placeholder glyphs, camera effects, production panels.
- History replay/historical views, event feed, Watch/auto-pause, significance.
- Persistence, save/load.
- Multithreaded ECS, IPC, separate process (CHRON-030 is in-process only).
- LLM, NLG, war, politics, religion, magic.

## Dependencies
- CHRON-020 complete (local tile data model that the tile layer renders).
- CHRON-029 complete (Render Snapshot DTO as the sole data source).
- CHRON-030 complete (Simulation worker command/snapshot path).
- CHRON-010 developer metrics conventions and CHRON-011 tile renderer baseline to reuse/refactor.
- CHRON-002 Godot project and ADR-0007 Godot bridge boundary.

## Execution Steps / Readiness

1. Check the actual 029/030 fields and lifecycle/ack API; parent records the
   thin Godot conversion contract, including lossless full-range EntityId.
2. Adapt existing tile renderer, add markers/controls/metrics from snapshots.
   Scene Tree may update presentation mirrors; it is not immutable UI, and
   none of those updates may drive Rust truth.
3. Add headless integration smoke and a separate windowed frame-capture path
   in the allowed Godot tools (§3/4). Capture 120 warm-up + at least 300 measured
   frames; distinguish base-terrain draw calls from whole-scene draw calls.
4. Parent verifies runtime fidelity, rejection feedback and M5 measurements.
   A script/UI leaf may be delegated; worker lifecycle/bridge review stays local.

## Files Modified / Allowed
- `apps/macos-godot/**` (scene, GDScript, renderer/sprite node, metrics overlay, time controls).
- `crates/godot-bridge/**` (presentation conversion from the Render Snapshot; the only Godot-dependent crate per ADR-0007).
- `apps/macos-godot/project.godot`, tile/atlas assets, and any presentation resources.
- `docs/reports/CHRON-031_GODOT_MICRO_WORLD.md` for the measured FPS result.
- `docs/tasks/CHRON-031.md`.
- Include this Task's necessary supporting files under P1-REMAINING §3: tests/fixtures, benchmark adapters, corresponding ADR and relevant architecture/performance/status documentation. Routine synchronization does not need a CP; Master Spec conflicts do. No `MASTER_SPEC.md` edits, unrelated refactoring or budget changes.

## API Contract
- Godot calls a bridge method `get_micro_world() -> Dictionary` (or the worker-driven equivalent) that returns the latest complete Render Snapshot; it is read-only and immutable from Godot's perspective.
- Godot issues commands via `command_worker(command)` mapping to `WorkerCommand`; normal UI exposes Pause/Resume/SetSpeed/Step. AdvanceTo stays a diagnostic/benchmark path. Enqueue failure and application acknowledgement are distinct UI states.
- The bridge must not expose any method that mutates kernel world state except through the worker's bounded command path.
- Presentation invariant: the Scene Tree holds only a presentation mirror; no node stores authoritative position/action truth used to drive simulation.
- The presenter must not invent a simulation value (e.g., scheduler depth) the snapshot does not provide; it labels unavailable fields as unavailable.
- A `snapshot_frame()` read must be cheap and batched; the client should not issue per-person calls in the hot path.

## Tests
- Snapshot fidelity: after N worker ticks, the Godot-presented persons/tiles/actions match the Rust snapshot for the same committed tick; no drift or invented values.
- Authority guarantee: changing/removing a presentation Node may alter the local drawing, but cannot change the Rust snapshot/truth. Only approved worker commands affect simulation.
- Time-control correctness: Pause freezes the presented `sim_second`; SetSpeed/Step change it only at tick boundaries; N-step advances by exactly the requested ticks.
- Metrics overlay: overlay values exactly mirror snapshot metrics; unavailable client-side fields are labeled unavailable, not fabricated.
- 60 FPS target: rendering 100 moving persons + full tile map sustains 60 FPS on the M5 reference machine (mirroring CHRON-011 method, discarding warm-up, ≥300 frames, p95/min/mean reported).
- GDScript static validation and zero runtime errors during the windowed run.
- Workspace Rust gates unaffected (core remains headless and Godot-independent); Godot-specific tests run on the macOS Godot CI job.

## Benchmark
- FPS / frame-time for 100 moving persons + 128×128 tile map, release Rust + windowed Godot on M5 16GB: discard a documented warm-up, capture at least 300 consecutive measured frames, and report min/mean/p95 FPS/frame time, draw calls, and video memory (mirroring CHRON-011 method).
- Report per-tick presentation latency and snapshot refresh cost, and confirm budget headroom for the 100-person Phase 1 hard gate.
- No budget relaxation; if 60 FPS is not sustained, report the gap and limitation rather than silently lowering the target.

## Definition of Done
- The Godot client presents 100 moving persons and the tile map entirely from a read-only `RenderSnapshot`; Scene Tree holds only presentation state.
- Time controls (pause/speed/step) issue worker commands and reflect changes only at tick boundaries.
- A metrics overlay mirrors Rust snapshot metrics and labels unavailable client-side values as unavailable.
- The client sustains 60 FPS on the M5 reference machine for the 100-person micro world, with method and limitations documented.
- No Godot code mutates simulation truth; the bridge is the only Godot path and remains presentation-only.
- Rust Core remains headless and Godot-independent.

## Required Completion Report
Report: change summary; commands run; benchmark result (FPS min/mean/p95, frame time, draw calls, video memory, presentation latency) with any N/A restricted to genuinely inapplicable metrics, never missing mandatory evidence; list of covered tests; known limitations (e.g., placeholder art, no animation rigs, single worker in-process, no persistence/history); any editor-only crash recurrence (monitor the isolated exit case; stop affected acceptance for normal-runtime recurrence); and any blocker. Continue to the next verified-ready Task already covered by the approved plan; do not ask for routine reconfirmation.
