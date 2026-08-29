# CHRON-011 128×128 Tile Renderer Result

Date: 2026-08-29

## Environment

- Apple M5 MacBook Air, 10 cores, 16 GB RAM
- macOS 26.6.2
- Godot 4.7.2 stable official, Forward Plus, windowed at 1280×720
- Debug Rust GDExtension loaded

## Workload

A Godot 4.7 `TileMapLayer` rendered a 128×128 map containing exactly 16,384
cells. The spike generated a four-color 4×4-pixel atlas at runtime, disabled
collision, populated every cell once, and left the complete 512×512 map visible.
The purpose is renderer validation, not Phase 1 terrain semantics or final art.

## Result

The in-project sampler discarded 60 warm-up frames and recorded 300 rendered
frame deltas:

- Average FPS: **60.00**
- Slowest sampled frame: **60.00 FPS**
- p95 frame time: **16.667 ms**

An independent GDA 300-frame monitor window confirmed:

- FPS min / mean / p95: **60 / 60 / 60**
- Draw calls min / mean / p95: **1 / 1 / 1**
- Video memory: **36,356,096 bytes (34.67 MiB)**
- Visible canvas objects: **16,384**
- Runtime errors: **0**

The Phase 0 workload meets the Master Spec's normal-UI 60 FPS target on the M5
16 GB test machine. VSync capped this run, so the result proves target
sustainment rather than maximum uncapped rendering throughput.
