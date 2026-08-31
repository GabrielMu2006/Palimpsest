# CHRON-030–032 independent review closeout

Status: repair implementation and short correctness checks verified; full
corrected ten-year/RSS collection running. No final Phase 1 acceptance yet.
Owner instruction: fix the reviewed issues with Codex Luna, complete 033–036,
and avoid unnecessary repeated expensive RSS runs. Contract: ADR-0028.

## Findings and repairs

| Finding | Repair and independent evidence |
|---|---|
| Shutdown could keep applying already-queued later commands | Close command processing and reject later work; preloaded FIFO unit regression |
| Explicit AdvanceTo could monopolize worker until a ten-year target | Queued Pause/Shutdown interrupts at a complete kernel-call boundary, returns Interrupted at actual time; real worker regression |
| Separate snapshot/status reads could attach the wrong publication number | Atomic publication metadata plus paired observe API; concurrent read test and bridge integration |
| Worker benchmark could see acknowledgement after an earlier publication poll and panic | Read publication after acknowledgement and time the publication itself |
| Old FPS p95 inverted frame-time p95; `_process(delta)` produced synthetic constant intervals | Monotonic frame-start timestamps and 300 consecutive raw intervals; reciprocal-percentile negative test |
| Render tests mostly checked array sizes | Actual terrain/site nodes and windowed MultiMesh transforms/colors; intentional corruption must be detected |
| RSS, snapshot age and conversion cost were missing or inferred | Measured process maximum, construction/publication timestamps, and actual Rust dictionary-conversion duration |
| Every non-Idle activity completion was counted as movement | Count committed cell steps/real arrivals separately from top-level completions; zero-distance Work regression |
| CLI could not detect a call that never returned | Bounded heartbeat supervision; blocked-worker and panic negative tests; invalid-input checks |
| Actor-only references and partial report comparisons | Actor+target validation, person/action checks at returned boundaries, final next_due, full deterministic report equality/hash schema2 |

Luna delivered presentation and chaos leaves; parent reviewed both, requested one
bounded rework each, then took over remaining omissions. Parent added/repaired
worker synchronization, observation regressions, final CLI supervision and
full-report checks. No agent delivery was treated as acceptance on its own.

## Verified commands and limitations

- `cargo clippy --locked -p palimpsest-sim-core -p palimpsest-sim-scheduler
  -p palimpsest-headless-runner -p palimpsest-bench-memory
  -p palimpsest-godot-bridge --all-targets -- -D warnings`: passed.
- Debug tests for Core/Scheduler/runner: existing and new tests passed except
  the initial new publication test imposed an unjustified 250ms deadline.
  It now waits for actual publications, preserving pairing assertions and a
  60s deadlock ceiling; its targeted debug rerun passed. No product latency
  budget was changed and no existing assertion was removed.
- `cargo test --release --locked --workspace --all-targets --all-features`:
  passed; exact execution count recorded below. No failed/ignored tests.
- `gda script validate main.gd capture_statistics.gd tests/capture_statistics.gd
  tests/chron031_integration.gd --project apps/macos-godot --json`: all valid.
- `gda script run tests/capture_statistics.gd --strict ... --json` and
  `gda script run tests/chron031_integration.gd --strict ... --json`: passed.
- Native windowed Godot `--script res://tests/chron031_integration.gd`: passed,
  including corruption detection and restoration. Headless backend explicitly
  does not claim MultiMesh GPU readback fidelity.
- `chaos_memory --seconds 172800`: passed; 100/100 persons completed required
  movement/Eat/Sleep/Work. This is a two-day smoke, not the ten-year evidence.
- `bash -n tools/capture-frames.sh`, `cargo fmt --all`, `git diff --check`: passed.

The macOS capture wrapper needed parent corrections for BSD mktemp (Xs at end)
and Bash 3 empty-array behavior. Two failed launches are retained separately;
neither produced a valid frame sample. Successful captures are not replacements
selected for faster performance. No slow frame was discarded.

## Windowed capture

Release GDExtension, seed42, 100 persons, 100x pacing, 120 warmup frames then 300
consecutive records. Full-scene mean 59.9753 FPS, minimum reciprocal interval
47.7145 FPS, FPS p95 61.0724; frame p95 16.997ms and max20.958ms. This is roughly
60Hz presentation with measured jitter, not an exact 60 FPS guarantee for every
frame. Whole scene19 draw calls; terrain-only variant1 draw call. Full capture
process maximum RSS277,954,560 bytes, measured by `/usr/bin/time -l` around the
capture wrapper (build skipped); Core runs inside the Godot process. This is
whole-process high-water, not an incremental RSS measurement or allocated bytes.

Snapshot age p95 102.169ms, build p95 90us, Rust snapshot acquisition/dictionary
conversion p95 110us, whole snapshot_frame call p95 120us, node refresh p95 137us.
Age is observed at acquisition, not inferred from 10Hz. Timings include normal
OS scheduling; there is no hard100ms promise. The terrain-only capture showed
substantial timing variability (mean67.32 FPS, one134.731ms interval) and is only
a draw-call isolation diagnostic; no FPS or VRAM subtraction is justified.

Raw: `data/chron-031-repair-frames{,-minimal}.json` and `.time.txt`;
source/binary identity: `data/chron-030-032-repair-source.json`.

## Ten-year evidence policy

Original three chaos runs and original one `kernel-10-year` RSS sample remain
unchanged. The latter is valid for its colocated fixture and cannot be relabeled
as the spread chaos fixture. Exactly one corrected full chaos run combines native
peak RSS proof, daily current-RSS trend and corrected movement/reference checks.
Short deterministic repetitions and the old event/truth-work counters provide
cross-checks; no three additional ten-year runs are requested. RSS n=1 has no
variance estimate and does not prove absence of every possible leak.

Full-run result: pending; do not infer completion from this document.

Historical Phase0 reports and Master Spec hashes match the pre-review worktree.
No commit, push, Draft PR, merge, or remote-setting change has occurred at this
repair checkpoint. 034 owns planned candidate publication; Phase2 is excluded.

Release execution count: 391 passed, zero failed/ignored, across 44 test targets.

## Completed corrected ten-year observation

The frozen-source run completed in **1565.935157375 s** (one run, not a timing
variance estimate). All100 persons completed actual movement/Eat/Sleep/Work over
3650days, final time315360000, next_due315361289, no violated invariants.
Events/decisions6717900, transitions202290027, rounds7934674 and the retained
stream digest15183745062922387530 match the historical report exactly.
Real movement phases2239300 replace the old mislabeled completion count;
schema2 full report hash6301723614086087630 is intentionally not the old hash.

Native RSS: cold baseline1540096 B, peak6619136 B, proven increment5079040 B;
prepared baseline3293184 B, proven increment3325952 B. Both proofs are
baseline_at_lifetime_peak. Daily current RSS has3650 samples, range4636672–6619136 B,
first5193728 B and final5898240 B. It includes accumulating daily report rows and
allocator behavior; these observations do not prove absence of every possible leak.
Queue observations cover10950 returned advance boundaries: depth200 throughout,
heap nodes202–463. They are not instantaneous intra-call peaks.

Raw: [corrected chaos/RSS](data/chron-032-repair-chaos-memory.json),
[progress](data/chron-032-repair-chaos-memory.progress.txt),
[frozen source identity](data/chron-030-032-repair-source.json).
The earlier pending-ten-year notes describe the evidence state before completion.
No second or third long run is required by ADR0028/owner preference.

Measurement boundary clarification: Core native high-water is read at the completed
workload boundary, before the outer CLI's final JSON output encoding. It is not a
promise about every later adapter allocation. Godot's `/usr/bin/time -l` result
covers the whole windowed process. Neither serialized bytes nor RSS increments
are mislabeled as a full configuration's total memory footprint.
