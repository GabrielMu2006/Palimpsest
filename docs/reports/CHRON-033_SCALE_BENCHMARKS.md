# CHRON-033 — Representative scale benchmarks

Status: reference measurements complete and independently checked; CHRON034 owns hosted gates.
Machine: Apple M5 /16GiB /macOS26.6.2 /Rust1.98.0 /Godot4.7.2 /bevy_ecs0.19.1.
Source: [SHA256 source and binary manifest](data/chron-033-source.json), [environment](data/chron-033-environment.json), [freeze/equality verification](data/chron-033-freeze-verification.json).

## Method and correctness

`python3 tools/collect-chron033.py` invokes already-built release binaries sequentially.
Each scale uses seed42/default map/reachable distinct spawn cells, identical86400-second
horizon, two complete warmups and ten samples. All60runs passed population, actual
per-person movement/Eat/Sleep/Work, needs/terrain/queue/future-due validation; raw
snapshot hashes, work counters, event digests and returned-boundary queue samples
agree across every repetition. Every native-RSS snapshot hash also matches its timing run.
No concurrent agent build/test/benchmark ran during reference sampling.

Timing excludes construction and post-validation. Counters are prepared-to-final
deltas, not setup credited as timed work. Upper median and population variance are
recorded with all raw samples. Queue maxima are sampled at successful advance returns,
not instantaneous intra-call peaks. The path probe uses up to64evenly selected prepared
positions to actual Work sites; failed queries remain counted. Its elapsed time is
an isolated query batch, NOT an integrated pathfinding CPU share. Integrated attempted
candidate/execution queries are separately counted; no AI/cadence/LOD/gameplay change.

## Scale results

| Persons | Advance median s | min–max s | σ s | sim-s/wall-s | Native peak B | Cold increment B | Snapshot bytes | Build / serialize median µs |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 100 | 0.430765 | 0.427992–0.433872 | 0.002020 | 200573.5 | 6422528 | 4866048 | 154030 | 10.541 / 105.084 |
| 1000 | 4.306776 | 4.301304–4.351122 | 0.014495 | 20061.4 | 14843904 | 13287424 | 302386 | 83.125 / 222.125 |
| 3000 | 13.040591 | 13.016275–13.101480 | 0.029598 | 6625.5 | 27377664 | 25821184 | 627702 | 259.125 / 486.416 |
| 5000 | 22.128023 | 21.833624–22.280844 | 0.155792 | 3904.6 | 38420480 | 36864000 | 948891 | 438.833 / 703.958 |
| 10000 | 44.261398 | 43.686305–50.104266 | 1.822708 | 1952.0 | 64454656 | 62898176 | 1734575 | 899.375 / 1306.875 |

RSS uses one cold native process per scale, with both baseline-at-lifetime-peak
proofs valid in every cold/prepared interval. n=1 is not a variance claim. The
interval includes setup, advance, validation, final snapshot/JSON bytes and hash,
ending before outer adapter-output encoding. It is a Core/tool observation, not
the10K-client5GB configuration. Snapshot JSON bytes are not RSS; the static terrain
JSON payload is137205bytes at every scale. No sample or higher-scale failure was hidden.

## Work and separate probes

Rates below are deterministic work counts divided by the advance median, not standalone subsystem microbenchmarks.

| Persons | Scheduler enqueue / dequeue per s | Decisions / events per s | Candidate / execution path queries per s | Transitions per s | Queue depth / heap max | Probe calls, median µs |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 152786.4 / 144094.9 | 4345.8 / 4345.8 | 87806.6 / 4345.8 | 149854.4 | 200 / 449 | 64, 7977.334 |
| 1000 | 151909.0 / 143150.2 | 4379.4 / 4379.4 | 88381.0 / 4379.4 | 148970.4 | 2000 / 2662 | 64, 6882.250 |
| 3000 | 148808.9 / 140103.8 | 4352.6 / 4352.6 | 87796.6 / 4352.6 | 145927.4 | 6000 / 11635 | 64, 5967.625 |
| 5000 | 144132.8 / 135568.1 | 4282.4 / 4282.4 | 86357.5 / 4282.4 | 141319.6 | 10000 / 19537 | 64, 4933.875 |
| 10000 | 138101.7 / 129527.2 | 4287.3 / 4287.3 | 86439.2 / 4287.3 | 135325.3 | 20000 / 39428 | 64, 3460.917 |

All probes succeeded for64positions; individual path lengths and counter totals,
scheduler cancellations/reschedules, min/max/sum queue observations, terrain/person
payload bytes and snapshot work metrics remain in the raw files. Reschedules are
zero for this fixture; cancellations are real observed operations, not inferred updates.

## Matching100-person direct / worker / native windowed run

| Interval | Median ms | Ratio to direct median |
|---|---:|---:|
| Direct advance | 428.760250 | 1.000 |
| Worker submit → observed ack | 430.560916 | 1.0042 |
| Worker submit → publication timestamp | 430.207958 | 1.0034 |
| Godot submit → observed ack | 563.488 | 1.3142 |
| Godot submit → final target frame drawn | 596.728 | 1.3918 |

All direct/worker/rendered results end at86400seconds with diagnostic hash
14346005809762790435 and identical render DTO/work counters. This hash is a
non-cryptographic DTO comparison, not a full persistence-state archive. Godot used
the real main scene/worker,120engine-warmup frames and2+10fresh worlds. No1x pacing.
Worker polling is1ms; Godot confirmation is observed on a rendered frame. Publication
uses the worker timestamp, so it is not incorrectly inferred from an earlier poll.
Godot final target rendering adds a median33.309ms after acknowledgement observation.
Final-point snapshot read median16µs, bridge conversion14µs, build8µs, age48.709ms;
these are final-point reads, not a per-frame distribution. Actual frame counts are retained.

Windowed whole-process RSS high-water286244864B, including Core+Client. The100-person
observation is below the unchanged3GB cap. The10K headless peak64454656B is below5GB
as a Core-only observation; it does not certify a10K client. OptionalLLM7GB configuration
and NLG/history-query/fullrelationship/memory systems are NotApplicable inPhase1.
Worker/rendered comparison is the plan's explicit100-person comparison; higher scales
are direct-Core diagnostics, not measured worker/client configurations.

The new worker ratio is about1.0042; the Godot ack ratio about1.3142. The Phase0
dummy ratio~2.09 is historical, with different work and scheduling, and is not reused.
Roughly linear total advance cost across these populations is evidence for this fixture,
not a full-game scaling guarantee or proof of a particular internal bottleneck.

## Evidence and validation

Parent added the omitted raw sampling/probe/counter/summary details after one bounded
Luna leaf rework, and independently verified every result. Four focused Rust benchmark
tests pass; denied-warning Clippy passes for the touched packages. Native Godot smoke
passed before formal capture. The complete two-day chaos report matches the pre-counter
repair report exactly across two new short runs; no ten-year repeat is needed for this
observational-only change. CHRON034/036 record final workspace and hosted gates.

Raw data per scale:
- 100: [timings](data/chron-033-scale-100.json), [native RSS](data/chron-033-rss-100.json); adjacent invocation files contain exact commands/exits.
- 1000: [timings](data/chron-033-scale-1000.json), [native RSS](data/chron-033-rss-1000.json); adjacent invocation files contain exact commands/exits.
- 3000: [timings](data/chron-033-scale-3000.json), [native RSS](data/chron-033-rss-3000.json); adjacent invocation files contain exact commands/exits.
- 5000: [timings](data/chron-033-scale-5000.json), [native RSS](data/chron-033-rss-5000.json); adjacent invocation files contain exact commands/exits.
- 10000: [timings](data/chron-033-scale-10000.json), [native RSS](data/chron-033-rss-10000.json); adjacent invocation files contain exact commands/exits.
- [worker raw](data/chron-033-worker.json), [rendered raw](data/chron-033-rendered.json), [engine/time log](data/chron-033-rendered-engine.stderr.txt).
- [two-day observational regression](data/chron-033-observational-regression.json).

Budgets remain3/5/7GB and60FPS. Phase1 hard correctness is100persons/10years, not
the one-day scale benchmark. CHRON031 owns normal UI frame behavior and its disclosed jitter.
