# CHRON-031 — Godot Micro World Presentation

> Review update 2026-08-31: The original 60/60/60 FPS values used engine delta and are superseded. Correct monotonic 300-frame observations and native windowed fidelity verification are in CHRON-030_032_REVIEW_CLOSEOUT.md; normal UI mean59.975 FPS, p95 frame16.997ms, no constant-60 claim.
> Implemented and locally verified 2026-08-31 by the main agent (Kimi Code
> CLI). Contract: [ADR-0026](../adr/ADR-0026-phase-1-godot-presentation-contract.md).

## Change Summary

The Godot macOS client now presents the Phase 1 micro world entirely from the
Rust worker's published snapshots: 100 persons moving over the 128×128
terrain, time controls (pause/resume/1–1000×/MAX/step), and a developer
metrics overlay mirroring snapshot metrics. The Scene Tree holds presentation
mirrors only; nothing Godot-side mutates simulation truth.

- `crates/godot-bridge`:
  - New `src/frames.rs` — pure, engine-free conversion layer (ADR-0026):
    batched `FrameData` from `RenderSnapshot` + `WorkerStatus`; lossless
    full-range `EntityId` as 8 little-endian bytes (never `f64`/`i64`);
    command parsing/validation; ack wire mapping. Unit-tested.
  - `src/lib.rs` — new `PalimpsestMicroWorld` Godot class: `create_world`
    (decimal-u64 seed, 1–100 persons), `snapshot_frame()` (single batched
    read), `command()` / `command_status()` (enqueue vs acknowledgement are
    distinct states). The Phase 0 `PalimpsestBridge` spike class is untouched
    (its retirement is CHRON-035).
  - `Cargo.toml`: added path edges to `palimpsest-sim-ai` and
    `palimpsest-sim-world` (inward domain crates; Cargo.lock gained only
  these two local package edges — no third-party change).
- `apps/macos-godot`:
  - `main.gd` rewritten: creates the world (seed 42, 100 persons), reads one
    `snapshot_frame()` per frame, mirrors it into the tile/person/overlay
    nodes, polls command acks, keyboard shortcuts (space/period/1–6), and the
  `--capture-json=PATH` / `--capture-minimal` frame-capture mode.
  - `tile_renderer.gd` rewritten: terrain batch from snapshot bytes
    (Ground/Water/Rock atlas), static-site markers; no procedural terrain.
  - New `person_renderer.gd`: all persons as one MultiMesh (one draw call),
  colored by observable action state.
  - `metrics_overlay.gd` rewritten: SIMULATION section mirrors snapshot/worker
    metrics verbatim; CLIENT section labelled rendering-only; LOD labelled
  unavailable rather than fabricated.
  - New `time_controls.gd`: Pause/Resume/Step+1s/Step+10s/speed buttons with
    distinct enqueue/ack feedback.
  - `main.tscn`: Persons node + TimeControls added; `project.godot`
    unchanged.
  - New `tests/chron031_integration.gd` headless integration test.
- New `tools/capture-frames.sh`: release GDExtension, windowed run, 120
  warm-up + 300 measured frames, JSON report, error-scan.
- ADR-0026 recorded before implementation.

## Commands Actually Run

```sh
cargo build -p palimpsest-godot-bridge          # debug dylib for dev/smoke
cargo test -p palimpsest-godot-bridge           # 3 conversion unit tests
cargo clippy -p palimpsest-godot-bridge --all-targets -- -D warnings
gda script validate --all --project apps/macos-godot --json    # 5/5 valid
gda scene validate main.tscn --project apps/macos-godot --json # valid
gda script run tests/chron031_integration.gd --project apps/macos-godot --json
./tools/ci-godot.sh                             # headless smoke, exit 0
./tools/capture-frames.sh docs/reports/data/chron-031-frames.json
# windowed minimal variant: godot --path apps/macos-godot -- --capture-json=... --capture-minimal
gda daemon start --windowed ... ; gda screen capture ; gda daemon stop ; gda daemon uninstall
./tools/ci-rust.sh                              # exit 0
cargo test --release --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo metadata --locked --no-deps --format-version 1
cargo test --release --locked -p palimpsest-bench-memory --test cli
git diff --check                                # clean
```

All passed. The gda daemon harness was installed for one screenshot and fully
uninstalled afterwards; `project.godot` verified byte-identical after
uninstall and `apps/macos-godot/addons/` removed.

## Benchmark Result (M5 16 GB, Godot 4.7.2 Forward+/Metal, windowed, release extension)

Method mirrors CHRON-011: 120 warm-up frames discarded, 300 consecutive
measured frames, world driven at 100× so all 100 persons visibly move
(snapshot publications advanced every frame batch: publication counter 64 at
capture end, sim reached 666 s). Raw data:
`docs/reports/data/chron-031-frames.json` (full scene) and
`chron-031-frames-minimal.json` (base terrain only).

| Metric | Full scene (terrain + 100 persons + sites + UI) | Base terrain only |
|---|---|---|
| FPS min / mean / p95 | 60.0 / 60.0 / 60.0 | 60.0 / 60.0 / 60.0 |
| Frame time mean / p95 / max | 16.667 ms all | 16.667 ms all |
| Draw calls min / mean / p95 | 19 / 19.0 / 19 | 1 / 1.0 / 1 |
| Video memory p95 | 57,245,696 B (54.6 MiB) | 32,161,792 B (30.7 MiB) |

VSync caps the run at 60 FPS: this proves the 60 FPS target is sustained with
zero missed frames, not uncapped headroom. Base terrain stays at the CHRON-011
1-draw-call target; the 100-person MultiMesh adds exactly 1 draw call; the
remaining 17 are site markers and UI.

Presentation latency: the client reads the newest complete publication each
frame; the worker publishes at 10 Hz while running (CHRON-030), so a rendered
snapshot is at most ~100 ms of wall clock behind the last committed boundary
at speed, and exactly current after pause/step (forced publication).
Per-tick presentation cost: one batched `snapshot_frame()` dictionary per
frame; measured implicitly by the 60 FPS capture (no per-person calls exist).

## Test Coverage

- Bridge unit tests (3): full-range `u64` EntityId byte round-trip including
  `u64::MAX` and `i64::MAX+1`; command validation before the worker;
  terrain/site/action encodings match ADR-0026.
- Headless integration (`tests/chron031_integration.gd`, 34 assertions, all
  pass): world creation + rejection of duplicate/bad-seed creation; initial
  frame shape (schema 2, epoch, 16,384 cells, 100 persons, lossless unique
  non-zero byte ids, in-range coords/states, sites present, metrics mirror);
  enqueue-vs-ack distinction (invalid commands rejected at the bridge,
  unpaused step enqueued then rejected in its ack); exact 10-second step and
  forced publication; running advance at 1000×; pause freezes the presented
  `sim_second`; presentation mutation never reaches the snapshot; shutdown
  closes the command path.
- GDScript static validation: 5/5 scripts valid; `main.tscn` validates.
- `ci-godot.sh` headless smoke: extension loads, main scene runs, zero
  errors. Zero runtime errors during both windowed captures.
- Workspace Rust gates all pass (`ci-rust.sh` exit 0 incl. deny-warnings
  clippy, MSRV 1.95, seven smokes; release tests; doctests; rustdoc;
  dependency metadata/tree review — only the two new inward path edges).
- Visual confirmation: windowed screenshot inspected (terrain, site markers,
  persons, overlay, controls render as intended).

## Known Limitations

- Placeholder presentation: solid-color tiles/quads, no spritesheets or
  animation rigs (out of scope); persons spawn colocated at the first
  walkable cell (the approved kernel fixture), so at the epoch they overlap
  on one tile until the world runs.
- 60 FPS is vsync-capped sustainment, not uncapped headroom; no FPS claim
  beyond the 100-person Phase 1 micro world.
- The snapshot id encoding is verified lossless by unit/integration tests;
  GDScript displays ids only via byte-wise hex (never reassembled into a
  Godot `int`), so no client-side id arithmetic exists to lose range.
- Single in-process worker; no IPC/multi-threaded ECS; no persistence,
  history, event feed, Watch, or auto-pause (later tasks).
- `tools/capture-frames.sh` requires a windowed macOS desktop session; CI
  keeps the headless smoke only.
- Work remains uncommitted on `phase-1-planning` per the plan (publication is
  CHRON-034); hosted CI has not run on this tree.

## Source Identity (SHA-256, 2026-08-31)

| File | SHA-256 |
|---|---|
| crates/godot-bridge/Cargo.toml | `e0a818c90466267d28d8933345312f6058e3d1dc08950d64b4887c75ffcd4ab8` |
| crates/godot-bridge/src/lib.rs | `74f4ed28bf9492edf78e0cf84e6605d0600afd7d776c6bd4978ce1919b6d36a0` |
| crates/godot-bridge/src/frames.rs | `8d7ea89f8376cab00ab48860b0d638c426c59436e509c65414422d3102779d6f` |
| apps/macos-godot/main.gd | `42a727b501142c053795c3f6f9cd46dc3c36b524943cbab6e63a96760ef6de70` |
| apps/macos-godot/main.tscn | `2f6e69fa3ee255e14ad1076e6f7c322f34b3f2664a33712adccda06d19993c29` |
| apps/macos-godot/tile_renderer.gd | `824d90ca33f38b681edded3253a52640f59466c772fe041a5565ad0c736c6be8` |
| apps/macos-godot/person_renderer.gd | `79fe9a488f39c023c011af1115daa91bee7e863a56cbcaaa33dbc7825648220b` |
| apps/macos-godot/metrics_overlay.gd | `5e9c0c79f43daadfa930ed6e58c48394e076b6a10dbc98c7aa793b4ed7bba328` |
| apps/macos-godot/time_controls.gd | `77ee1406dcba4911f8d365dc9965b80e387edcafc698fe742d7dceeb1b947b33` |
| apps/macos-godot/tests/chron031_integration.gd | `887f4b9c2485d936a1b06f35f0b373e53b32d5b07d79ab5aa6a40a9b42aa7c8e` |
| tools/capture-frames.sh | `10c802d0920f61415591ab112d990f196cc92f1dad080cd2b686bd2f631d7bdb` |
| docs/adr/ADR-0026-phase-1-godot-presentation-contract.md | `ed58be0fcab4762686d68db7f6687adc7108f88252bddeb7ec36ba7a8fecbab0` |
| Cargo.lock | `ca57c978685d1b55c22bceae24c1c4c156019f8ca8010f2f8249eb73bba94696` (two local path edges only) |
| MASTER_SPEC.md | `a6fa0654…` (unchanged, read-only) |

## Next Ready Task

CHRON-032 (headless 10-year chaos runner, depends on CHRON-028) is the next
task in the approved DAG; it is **not** authorized by this report and was not
started.

## Final short presentation observation

After CHRON033's observational query counters, one short normal-UI capture on
candidate8dc1595 recorded300consecutive frames after120warmups: mean60.002088FPS,
min38.974199FPS,p95FPS61.236987; frame mean16.666087ms,p9517.008ms,max25.658ms.
Draw calls19; snapshot age p95102.451ms, build90µs, bridgeconversion125µs,
fullsnapshotcall139µs, nodeupdate154µs. Whole-process high-water278315008B.
Both corrected captures remain visible; this is not a rerun-until-pass policy.
Only the short normal capture was repeated after instrumentation, not minimal
rendering or the ten-year run. [Raw](data/chron-031-final-frames.json),
[source](data/chron-031-final-frame-source.json), [time/RSS](data/chron-031-final-frames.time.txt).
Subsequent spike retirement removes unused APIs; the normal presentation path
and simulation rules are unchanged. No constant60FPS claim is made.
