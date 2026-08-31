# ADR-0026: Phase 1 Godot Presentation / Frame Conversion Contract

- Status: Accepted — covered by the product owner's explicit CHRON-031
  implementation instruction on 2026-08-31
- Date: 2026-08-31
- Decision owners: Product owner
- Task: CHRON-031 in `docs/tasks/CHRON-031.md`; worker/DTO semantics per
  ADR-0015 supplement, ADR-0022–0025
- Extends: ADR-0007, ADR-0015, ADR-0023; does not supersede the Master Spec

## Context

CHRON-030 delivers the worker and the immutable schema-2 `RenderSnapshot` in
Rust. CHRON-031 must present it in Godot without the Scene Tree becoming
simulation truth, with stable `EntityId` values crossing the bridge
**losslessly over the full `u64` range** (P1-REMAINING D3: never through
`f64`; the `u64::MAX` / above-`i64::MAX` boundary must be tested), one batched
read per frame instead of per-person calls, and a command path whose enqueue
failure and application acknowledgement are visibly distinct states.

## Decision

### 1. One batched frame read

`PalimpsestMicroWorld` (new `RefCounted` Godot class in `godot-bridge`) owns
the `SimulationWorker`. The only per-frame read is:

```text
snapshot_frame() -> VarDictionary
```

a single batched, read-only dictionary built from the worker's latest
published snapshot plus worker status. No per-person bridge calls exist.

Field encoding (all integers, no floats carrying identity):

- `schema_version: i64` (= 2), `sim_second: i64`, `publications: i64`.
- `terrain: PackedByteArray` — 16,384 row-major cells, `Ground=0, Water=1,
  Rock=2`.
- `site_x`/`site_y`/`site_kind: PackedInt32Array` — parallel arrays,
  `Meal=0, Rest=1, Work=2`.
- `person_id: PackedByteArray` — **8 little-endian bytes per stable
  `EntityId`**, the lossless full-`u64` encoding; GDScript displays ids
  through byte-wise hex formatting and never reassembles them into a
  Godot `int` (which is `i64`).
- `person_x`/`person_y`/`person_action`/`person_state`/`person_target_x`/
  `person_target_y: PackedInt32Array` — parallel arrays; actions
  `Idle=0, Move=1, Eat=2, Sleep=3, Work=4`; states `Idle=0, Moving=1,
  Eating=2, Sleeping=3, Working=4`; target coordinates use `-1` for `None`.
- `metrics: VarDictionary` — the schema-2 `RenderMetrics` plus
  `live_actions`, copied verbatim; and `worker: VarDictionary` — phase,
  speed, committed instant, publications, applied/rejected command counts,
  queue depth and max queue depth.

The conversion layer is split into pure Rust functions over plain vectors
(unit-tested off-engine, including the `u64::MAX` id boundary) and a thin
`#[func]` layer that only copies them into Godot containers.

### 2. Command path and feedback

```text
command(cmd: VarDictionary) -> VarDictionary        # {ok, sequence?, error?}
command_status(sequence: i64) -> VarDictionary      # {status, outcome?, committed_to?}
```

- `cmd.type` is one of `pause`, `resume`, `set_speed`, `step`,
  `advance_to`, `shutdown`; `set_speed.value` accepts only
  1/5/20/100/1000/`"max"`. Unknown types and values are rejected with a
  typed error string; nothing reaches the worker on rejection.
- `ok=false` means the command was never enqueued (validation, `Full`,
  `Closed`). `ok=true` returns the `sequence`; application or rejection is
  observed only through `command_status`, which maps to
  `pending`/`completed(applied|rejected:<error>)`/`evicted`/`unknown`.
  The UI must show enqueue failure and application acknowledgement as
  distinct states.
- `advance_to` is a diagnostic/benchmark command and is not exposed on the
  normal UI buttons.
- Command sequences cross as `i64` (the worker assigns from 1 upward; the
  `u64` sequence space cannot be exhausted by any real session). EntityIds,
  which can genuinely span the full `u64` range, never use this path — they
  use the byte encoding above.

### 3. Presentation authority

- The bridge exposes no method that mutates kernel state outside the worker's
  bounded command path. World creation is `create_world(seed_text: String,
  persons: i64)` with a decimal-`u64` seed (lossless input) and
  `1 <= persons <= 100` for this task's scope.
- Godot nodes hold presentation mirrors only: removing or editing a person
  marker node changes local drawing, never the snapshot; the next frame read
  restores the truth-derived view.
- Fields the snapshot does not provide (e.g. client FPS, draw calls, VRAM)
  are labelled client-side; fields neither side has are labelled
  `unavailable`, never fabricated zeros.

## Consequences

- GDScript never needs per-entity bridge traffic; one dictionary read per
  frame carries terrain once and persons every frame.
- The byte-encoded id keeps the door open for full-range persistent identity
  in later phases without a bridge contract change.
- Frame capture runs windowed with a release GDExtension build; the headless
  CI smoke keeps using the debug build and does not measure FPS.

## Rejected / Deferred Alternatives

- `f64` or `i64` EntityId transport: rejected; lossy or range-limited
  (P1-REMAINING D3).
- Per-person bridge calls: rejected; violates the batched-read requirement.
- Exposing `AdvanceTo` on normal UI: deferred; it remains a
  diagnostic/benchmark command per the CHRON-030 contract.
- Godot-side interpolation/prediction between snapshots: rejected for
  Phase 1; presentation mirrors the latest complete boundary exactly.

## Task Completion / Acceptance Gate

- Files: `crates/godot-bridge/**`, `apps/macos-godot/**`, frame-capture tool,
  tests, report, and the documentation sync listed in CHRON-031.
- Tests: Rust unit tests for the pure conversion (including `u64::MAX`),
  GDScript static validation, headless fidelity/authority/time-control
  integration, and the windowed 120-warm-up + ≥300-frame capture.
- DoD: per `docs/tasks/CHRON-031.md`; 60 FPS is a measured target, not a
  relaxed one.
