# CHRON-016 Event Throughput

- Date: 2026-08-29
- Hardware: Apple M5, 16 GB unified memory, arm64 macOS
- Rust: 1.98.0 stable, release thin-LTO
- Workload: 100,000 validated `dummy_update` events, one stable actor each, bounded significance, compact JSON
- Samples: ten after an unreported three-sample warm-up

## Results

- Generation min / median / max: 2.640 / 2.747 / 3.587 ms
- Median validated generation throughput: 36,408,876 events/s
- Serialization min / median / max: 15.899 / 16.028 / 16.302 ms
- Median JSON serialization throughput: 6,239,049 events/s
- Serialized bytes: 21,266,725
- Mean serialized size: 212.67 bytes/event

Timed-process observation:

- Maximum RSS: 24,182,784 bytes (23.06 MiB)
- Peak memory footprint: 23,560,552 bytes (22.47 MiB)
- Swaps/page faults: 0

RSS includes the retained vector of 100K structured events, strings, vectors, allocator state, executable, and serialization buffers. It is not steady-state Event Store memory.

## Limits

- The event mix is deliberately simple and does not approximate final metadata richness.
- This measures generation/validation and JSON serialization separately; it contains no SQLite I/O or durability cost.
- Event persistence throughput is reported only by CHRON-013 with exact WAL and transaction settings.

