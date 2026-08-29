# CHRON-013 SQLite Event Store Prototype

- Date: 2026-08-29
- Hardware: Apple M5, 16 GB unified memory, arm64 macOS
- SQLite: bundled through rusqlite 0.40.2
- Settings: WAL, synchronous NORMAL, foreign keys ON
- Workload: 100,000 compact structured dummy events, fresh database per run, release build

## Results

| Batch size | Append time | Events/s | Checkpoint | Final DB size |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 1.301 s | 76,883 | 0.343 ms | 28,692,480 bytes |
| 100 | 149.101 ms | 670,685 | 1.141 ms | 28,692,480 bytes |
| 1,000 | 119.334 / 119.714 / 119.676 ms | median 835,593 | 0.574–0.617 ms | 28,692,480 bytes |
| 10,000 | 117.649 ms | 849,984 | 0.049 ms | 28,692,480 bytes |

- Database growth: 286.92 bytes/event after WAL checkpoint.
- Every run asserted exact count and `PRAGMA integrity_check = ok`.
- Duplicate-ID integration tests prove whole-batch rollback; reopen/checkpoint tests pass.

## Interpretation and Limits

Batching materially improves throughput. A 1,000-event transaction is the current balanced prototype baseline; the difference from 10,000 is small in this workload. `synchronous=NORMAL` is an explicit durability/performance choice and is not equivalent to FULL. Events contain no causes or rich metadata here, and query throughput is not yet measured. Final schema normalization and retention remain future work.

