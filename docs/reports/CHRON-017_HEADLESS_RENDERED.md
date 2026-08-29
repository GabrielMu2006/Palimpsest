# CHRON-017 Headless / Rendered Mode Comparison

Date: 2026-08-29

## Method

Both modes called the exact same release-built
`palimpsest_sim_core::run_spike_workload` function. Each sample allocated 10,000
stable IDs, scheduled 10,000 work items across simulation seconds 0–1,000,
processed and validated 10,000 structured events, advanced the same `SimClock`,
and asserted an empty final Scheduler.

The standalone harness produced seven independent ten-sample medians. The
Rendered harness produced three independent ten-sample medians after the
128×128 TileMap had sustained its warm-up/sample window in a live Godot process.
Rendered trials were allowed to run without per-frame GDA polling; earlier
poll-instrumented trials were discarded after the probe was shown to perturb the
result.

## Result

- Headless representative median: **1.402209 ms**
- Headless throughput: **7.132 million entity-work items/s**
- Rendered representative median: **2.927709 ms**
- Rendered throughput: **3.416 million entity-work items/s**
- Headless elapsed-time advantage: **2.09×**
- Headless throughput advantage: **108.8%**
- Rendered throughput relative to Headless: **47.9%**

The conclusion is that the Architecture Spike benefits materially from keeping
historical simulation independent of rendering. This does **not** mean full NPC
simulation will achieve these rates: the workload has identity allocation,
scheduling, clock advancement, and structured-event validation, but no needs,
movement, psychology, ecology, politics, or persistence I/O.

## Limitations

The Rendered benchmark synchronously blocks Godot's main thread while each Rust
sample runs. It measures process/render-environment interference, not the future
threading or frame-budget policy. Independent trial medians varied noticeably,
so the ratio is an architecture-spike indicator rather than a regression gate.
