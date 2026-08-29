# CHRON-011 — 128×128 Tile Renderer

## Context
Phase 0 must prove the Godot client can render a complete 128×128 Local map on the M5 16 GB target.
## Scope
One Godot 4.7 `TileMapLayer`, a generated four-tile atlas, 16,384 populated cells, windowed runtime validation, FPS sampling, and a benchmark report.
## Out of Scope
World generation, gameplay terrain semantics, streaming chunks, animation, camera controls, art production, and Phase 1 Local Tile rules.
## Dependencies
CHRON-002 and CHRON-003.
## Files Modified / Allowed
Godot scene/scripts, this task, and `docs/reports/CHRON-011_TILE_RENDERER.md`.
## Tests
GDScript validation, scene validation, headless preflight, live windowed node inspection, exact cell count, runtime diagnostics, and screenshot inspection.
## Benchmark
After 60 warm-up frames, collect 300 rendered frame deltas with the complete TileMapLayer visible. Report average FPS, slowest-frame FPS, and p95 frame time.
## Definition of Done
Godot visibly renders all 16,384 cells and sustains the Master Spec's normal-UI 60 FPS target on the M5 16 GB machine. **Complete.**
