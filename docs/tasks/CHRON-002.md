# CHRON-002 — Godot 4 macOS Project

## Context

Phase 0 needs a minimal Godot 4 macOS client shell before the Rust GDExtension boundary can be tested. Godot is presentation-only and does not own simulation state.

## Scope

- Install and record a supported stable Godot 4 Standard build for macOS.
- Create `apps/macos-godot` with a minimal project configuration and launch scene.
- Configure a 1280×720 window using the Forward+ renderer as the initial macOS baseline.
- Validate the project, scene, and empty client startup through gda and Godot.

## Out of Scope

- Rust/Godot GDExtension or any bridge-facing API.
- Simulation state, `EntityId`, `SimClock`, Scheduler, events, storage, or snapshots.
- Tile rendering, developer metrics overlay, gameplay UI, assets, or export packaging.
- Signing, notarization, release builds, or benchmark claims.

## Dependencies

- CHRON-001 complete.
- Godot 4.6 or newer for all gda live capabilities; install the current supported stable Standard build.
- `gda` 0.12.0.

## Files Modified / Allowed

- `apps/macos-godot/**`
- `docs/tasks/CHRON-002.md`
- `docs/TOOLING.md`

The Godot application may be installed outside the repository at `/Users/gabrielmu/Applications/Godot.app`.

## API Contract

No simulation or bridge API is introduced. The initial scene contains presentation-only nodes.

## Tests

- `gda info --project apps/macos-godot --json`
- `gda project info --project apps/macos-godot --json`
- `gda scene validate main.tscn --project apps/macos-godot --json`
- `gda scene preflight main.tscn --project apps/macos-godot --json`
- Headless project launch with `--quit-after`.
- Windowed launch smoke test where the current desktop session permits it.
- Re-run CHRON-001 Rust formatting, lint, and tests to confirm client isolation.

## Benchmark

Not applicable. Tile FPS and Godot/Rust call overhead belong to later Phase 0 tasks.

## Definition of Done

- The pinned Godot build is provenance-checked and reports a supported version through gda.
- `apps/macos-godot/project.godot` opens with `main.tscn` as its main scene.
- Static scene validation and dynamic headless preflight pass without script/runtime errors.
- A macOS client window can be launched, or an environment-specific window-server blocker is captured precisely.
- No simulation truth or later Phase 0 feature is implemented in Godot.
