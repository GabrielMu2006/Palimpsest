# CHRON-012 Snapshot Prototype

- Date: 2026-08-29
- Hardware: Apple M5, 16 GB unified memory, arm64 macOS
- Codec: bincode 2.0.1 Serde + zstd 0.13.3 level 3
- Workload: 10,000 stable entity DTOs plus 10,000 pending-work DTOs, allocator and clock
- Samples: ten after an unreported warm-up

## Results

- Raw bincode: 248,259 bytes
- Stored snapshot including magic: 46,702 bytes
- Compressed/raw ratio: 18.81% (81.19% reduction)
- Encode min / median / max: 0.566 / 0.964 / 1.562 ms
- Decode + validate min / median / max: 0.517 / 0.863 / 1.137 ms
- Timed-process maximum RSS: 5,980,160 bytes (5.70 MiB)
- Timed-process peak memory footprint: 5,079,400 bytes (4.84 MiB)

Every sample decoded, validated, and compared the complete restored domain snapshot with the source.

## Limits

This is a Phase 0 format, not a compatibility promise. It contains dummy state, not a full ECS world. Decompression-size limits, schema migrations, deltas, content/simulation versions, and untrusted-file hardening are required before production saves. Runtime ECS handles and Scheduler heap nodes are deliberately absent.

