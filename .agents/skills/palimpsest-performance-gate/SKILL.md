---
name: palimpsest-performance-gate
description: Measure and gate Palimpsest performance, scale, LOD, memory, or throughput changes. Use when benchmarking or making a performance claim; do not trigger for ordinary functional changes without one.
---

# Palimpsest Performance Gate

Measure first; do not optimize from intuition. Use Apple Silicon M5 with 16 GB unified memory as the primary reference hardware and evaluate scale gates at 100, 1K, 3K, 5K, and 10K entities as applicable.

Record a reproducible baseline and result for wall time, simulation throughput, memory or RSS, events per second, snapshot size and latency, database throughput, and Godot bridge overhead.

Performance changes require before-and-after measurements using the same scenario, seed, build mode, and hardware context. Report variance and limitations. Reject gains that sacrifice history fidelity, stable identity, causality, or correctness. Reference `MASTER_SPEC.md` and `docs/PERFORMANCE.md` when it exists; do not loosen budgets without product-owner approval.
