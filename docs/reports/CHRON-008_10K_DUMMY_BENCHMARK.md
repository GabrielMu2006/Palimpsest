# CHRON-008 10K Dummy Entity Benchmark

- Date: 2026-08-29
- Hardware: Apple M5, 10 cores, 16 GB unified memory, arm64 macOS
- Rust: 1.98.0 stable; workspace MSRV 1.95
- ECS: `bevy_ecs` 0.19.1
- Profile: release, thin LTO
- Workload: two components (`EntityId`, 12-byte dummy state), separate stable-to-runtime HashMap, 1,000 full-query update steps
- Samples: five per scale after an unreported warm-up

## Results

| Entities | Updates | Elapsed min / median / max | Median entity updates/s |
| ---: | ---: | ---: | ---: |
| 100 | 100,000 | 0.096 / 0.104 / 0.116 ms | 964,636,429 |
| 1,000 | 1,000,000 | 0.819 / 0.872 / 0.929 ms | 1,147,062,602 |
| 3,000 | 3,000,000 | 2.257 / 2.339 / 2.438 ms | 1,282,508,381 |
| 5,000 | 5,000,000 | 3.728 / 3.740 / 4.102 ms | 1,337,077,149 |
| 10,000 | 10,000,000 | 7.824 / 7.873 / 7.905 ms | 1,270,116,905 |

Every sample asserted exact component update counts, stable-map cardinality, runtime-handle resolution, and a deterministic checksum.

## Memory Observation

`/usr/bin/time -l` was run against the same release binary:

- Zero-entity process maximum RSS: 1,769,472 bytes
- 10K-entity process maximum RSS: 4,177,920 bytes
- Observed delta: 2,408,448 bytes (2.30 MiB), approximately 241 bytes/entity
- Zero-entity peak footprint: 999,736 bytes
- 10K peak footprint: 3,408,208 bytes
- Swaps/page faults: 0

The delta includes ECS archetype storage, HashMap capacity, allocator effects, world bookkeeping, and benchmark state. It is not a precise per-component heap allocation and is far below the complexity of a real NPC.

## Interpretation

The measured dummy workload does not reject `bevy_ecs`. Identity separation works without serializing runtime handles, and the baseline is comfortably inside the provisional 10K memory budget. Continue the hypothesis through Phase 0, but do not extrapolate these simple tight-loop updates to AI, relationships, events, pathfinding, or history storage.

