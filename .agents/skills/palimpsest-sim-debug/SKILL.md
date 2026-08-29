---
name: palimpsest-sim-debug
description: Diagnose long-running Palimpsest simulation anomalies such as extinction, resource explosions, stuck NPCs, relationship corruption, event storms, slowdowns, memory growth, or implausible historical outcomes. Use when causal simulation debugging is needed, not for ordinary compile errors.
---

# Palimpsest Sim Debug

Find the first divergence rather than guessing from a late surprising result.

1. Reproduce the anomaly.
2. Reduce it to the smallest seed and scenario that still fails.
3. Run headlessly and collect relevant metrics.
4. Bisect simulation time to locate the first abnormal state.
5. Identify the first bad event or state transition.
6. Trace its causal inputs and owning system.
7. Fix the cause without weakening identity, causality, history fidelity, or correctness.
8. Add a regression test using the minimal reproduction.
9. Rerun the focused case and the relevant long simulation.

Record the seed, configuration, first-divergence time, event/state evidence, metrics, fix, and verification. Do not infer the cause solely from an outcome such as “year 200 looks wrong.”
