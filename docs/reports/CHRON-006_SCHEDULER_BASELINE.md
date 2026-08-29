# CHRON-006 Scheduler Baseline

- Date: 2026-08-29
- Hardware: Apple M5, 10 CPU cores, 16 GB unified memory
- Architecture: arm64 macOS
- Rust: 1.98.0 stable
- Profile: Cargo `release`
- Payload: `usize`
- Due-time distribution: `index % 10_000` integer simulation seconds

## Method

The harness schedules every payload into a fresh `Scheduler<usize>`, then pops every item with `SimInstant::MAX`. Correctness assertions require the popped count to equal the inserted count. One three-sample 100K process warm-up was run and discarded. Each reported size then ran ten samples in one process; results below use the median and include observed min/max wall-time ranges.

Command:

```text
cargo build --release --example scheduler_bench -p palimpsest-sim-scheduler
target/release/examples/scheduler_bench <ITEMS> 10
```

## Throughput Results

| Items | Enqueue min / median / max | Median enqueue ops/s | Dequeue min / median / max | Median dequeue ops/s |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 27.500 / 29.375 / 48.000 µs | 34,042,553 | 36.500 / 39.042 / 43.375 µs | 25,613,442 |
| 10,000 | 222.083 / 227.875 / 311.667 µs | 43,883,708 | 504.708 / 512.708 / 524.167 µs | 19,504,279 |
| 100,000 | 1.660 / 1.853 / 2.750 ms | 53,971,405 | 5.728 / 6.282 / 6.981 ms | 15,918,181 |

## Process Memory Observation

Command:

```text
/usr/bin/time -l target/release/examples/scheduler_bench 100000 10
```

- Maximum resident set size: 13,058,048 bytes (12.45 MiB)
- Peak memory footprint reported by macOS: 12,468,584 bytes (11.89 MiB)
- Timed-run median: 59,697,038 enqueue ops/s; 17,690,373 dequeue ops/s
- Swaps and page faults: 0

This is whole-process peak memory across ten sequential samples, including executable, allocator, runtime, vectors, heap, hash map, and measurement harness. It is not a queue-only allocation number and must not be reused as the 10K Entity RAM result.

## Interpretation and Limitations

- The heap baseline is comfortably above the current Phase 0 scheduling needs for dummy workloads, but it does not represent full simulation-system cost.
- The benchmark contains no cancellation or rescheduling churn; those paths are covered for correctness and require a later workload-specific performance scenario if they become dominant.
- Results are one local run, not a before/after optimization claim. Thermal state, process scheduling, and allocator reuse explain some min/max spread.
- No product performance budget was changed, and no history, identity, causality, or correctness behavior was removed for these results.
