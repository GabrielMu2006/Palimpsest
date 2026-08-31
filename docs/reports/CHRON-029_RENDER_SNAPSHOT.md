# CHRON-029 — Render Snapshot DTO

> Historical implementation/pilot report. Current schema2 validation contract:
> ADR-0024/0025; final local verification/measurement: [repair V2 report](P1_KERNEL_REPAIR_V2.md).
> The old "None" blocker statement below is not current verification.

## Change Summary

Added the immutable, versioned render snapshot DTO, `RenderSnapshot`, as the
read-only presentation contract between the simulation core and the Godot
client.

- New module `crates/sim-core/src/render.rs` (ADR-0023), exporting from
  `crates/sim-core/src/lib.rs`:
  - `RenderSnapshot` (`schema_version`, `sim_second`, `terrain`, `persons`,
    `metrics`), `RENDER_SCHEMA_VERSION` (= 1).
  - `TerrainBatch` (row-major 128×128 `TerrainKind` cells + width/height),
    `PersonRender` (stable `EntityId`, `LocalCoord` tile, `ActionKind`, action
    target, `ActionState`), `RenderMetrics` (person count, scheduler queue
    depth, committed/buffered/rotated event counts).
  - `RenderSnapshot::from_kernel(&WorldKernel)` — the only constructor; reads
    the kernel's committed `now()`, terrain, persons (ascending by `EntityId`),
    and metrics without mutating the kernel and with no caller-supplied `now`.
  - `RenderError` and a `validate()` re-validated on deserialization (schema,
    cell count, non-zero/unique/ascending identity, metric count).
- New integration test `crates/sim-core/tests/render.rs` (7 tests).
- New benchmark example `crates/sim-core/examples/render_snapshot_bench.rs`.
- ADR-0023 (render DTO contract) recorded and accepted with the Task.

## Semantics Implemented (per ADR-0023, P1-REMAINING D3)

- The DTO uses only stable `EntityId`; no `bevy_ecs::Entity`, `ScheduleToken`,
  or runtime reference, and no `&mut` accessor or interior mutability.
- `TerrainBatch` is exactly the 128×128 local map; `PersonRender.person_id` is
  non-zero, unique, and the batch is ascending by `EntityId`.
- `sim_second` is `WorldKernel::now` at build time — a caller cannot inject a
  different instant.
- Deserialization (diagnostics only) re-validates schema/content/identity
  invariants; imported values are never written back into the world.
- Metrics carry only observable kernel fields; the DTO never fabricates an
  unmeasured value (wall-clock and RSS never appear).

## Commands Actually Run

```sh
cargo build -p palimpsest-sim-core
cargo test --workspace --all-targets --all-features        # all green, incl. 7 render tests
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo +1.95.0 check --workspace --all-targets --all-features           # MSRV 1.95 clean
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps              # clean
cargo fmt --all -- --check
cargo run --release --locked -p palimpsest-sim-core --example render_snapshot_bench -- --persons 100 --samples 1
```

## Benchmark Result

Release, Apple M5-class reference machine, one sample (pilot; the authoritative
two-warm-up/ten-sample M5 run is recorded in `docs/PERFORMANCE.md`):

```json
{"persons":100,"build_us":7.33,"serialize_us":116.12,
 "serialized_bytes":148878,"bytes_per_person":1489,
 "terrain_cells":16384}
```

Raw pilot output: `docs/reports/data/chron-029-render-snapshot.jsonl`.

## Test Coverage

- `snapshot_exposes_the_documented_schema_version`
- `snapshot_batches_the_full_local_tile_grid`
- `person_batch_carries_stable_identity_and_current_action`
- `snapshot_is_immutable_and_headless_boundary_safe`
- `snapshot_validates_schema_and_wire_invariants`
- `snapshot_never_invents_unmeasured_metrics`
- `malformed_wire_duplicate_or_zero_id_is_rejected`

## Known Limitations

- The DTO is a transient render view, not a save: no backward-compatible decode
  or save compatibility is promised (ADR-0009/0016).
- The bulk of the serialized size is the static 128×128 terrain batch (a fixed
  cost); per-person bytes are reported separately.
- Godot conversion belongs to CHRON-030/031; this Task fixes the headless DTO.
- Formal ten-sample DTO timing and RSS evidence is complete in repair V2 R2-05;
  GPU frame-budget correlation remains a later client task, not part of this repair.

## Blockers

None. Implementation is green and deterministic.
