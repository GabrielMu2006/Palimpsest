# ADR-0029 — Representative benchmark observations

- Status: Accepted design under the owner's CHRON-033–036 instruction;
  implementation starts only after repaired CHRON-032 is verified.
- Date: 2026-08-31.
- Extends ADR-0028; no gameplay, cadence, pathfinding algorithm, identities,
  persistence, history-retention or budget changes.

## Context and instrument inventory

Kernel/ActionRuntime already expose real decisions, transitions, completions,
events and successful scheduler operations. Neither CandidateContext nor
find_path exposes actual query counts. A movement completion is not a path
query: candidate enumeration and scoring can query repeatedly, and execution
queries again. Do not infer one from the other. Existing render snapshots are
immutable, Eq and Serialize; the worker and Godot bridge already support paused
AdvanceTo with final acknowledgement. Pacing comparisons are not equivalent work.

## Small read-only extensions

Keep CandidateContext Copy and its existing constructor. Add an optional borrowed
`Cell<u64>` query observer selected with `with_path_query_counter`; increment only
when the existing is_reachable path actually calls find_path, including failed
queries. It cannot change reachability, ordering, scoring or action selection.
ActionRuntime keeps candidate and execution query counts outside authoritative
ActionStats; expose `PathQueryCounts { candidate_queries, execution_queries }`.
WorldKernel forwards them through a fallible read accessor; faulted kernels
refuse live reads. Counters describe attempted work, not successful action commits.
No wall clock or global mutable counter enters simulation modules.

Add `RenderSnapshot::diagnostic_hash() -> u64`: fold canonical serde JSON bytes
using the already-used worker-benchmark multiplier 1,000,003 (wrapping arithmetic,
initial zero). This is a non-cryptographic render-state/work-counter comparison,
not a persistence format, full world-state archive, or collision-proof identity.
Godot exposes the latest published diagnostic hash as a decimal string, preserving
all64 bits, through a benchmark-only read method. It must be read after the
AdvanceTo acknowledgement and checked against the final simulated time.

## Scale and matching-work protocol

Use the existing seed42/default-map/default-kernel reachable spawn fixture for
all requested scales, same86400-second horizon. No hidden scale-specific rules.
If a higher scale cannot construct or finish its fixture, retain the failed
attempt and reason; do not silently choose an easier seed, map, or horizon.
100 is mandatory; higher scales are diagnostics, not guarantees of10K gameplay.

Two complete warmups and ten timed runs per scale. Each timed interval excludes
fixture setup and post-run validation/serialization; report work-counter deltas
from the prepared boundary so setup decisions/queries are not credited to the
advance timer. Validate population, needs/actions, required completions, queue
bounds, final time and future next_due. Store every raw sample and deterministic
comparison result. Serialize/build snapshots in distinct timed intervals with
bytes and fixed terrain/person payload sizes; do not mislabel bytes as RSS.

A separate bounded batch of real find_path calls uses evenly selected fixture
persons and reachable Work-site queries. Record calls, successes, path lengths,
and batch elapsed time. Label this an isolated query probe, never the summed
pathfinding share of the integrated kernel. Failed calls remain in its count/time.
Integrated candidate/execution counters independently prove the actual loop's work.

Direct/worker/windowed Godot use the same initial fixture and86400-second target,
with identical final snapshot diagnostic hash and work counters. Direct times
advance calls; worker times submit-to-observed-ack and submit-to-publication;
Godot runs the real scene while the unpaced explicit advance executes. Report
poll/frame quantization, snapshot conversion and rendered-frame counts separately.
No comparison uses1x pacing or extrapolates the old Phase0 dummy ratio.

Native cold RSS uses one separate process per scale under the owner's reduced
repeat preference (ADR-0028), retaining both whole high-water and proven
cold/prepared increments. n=1 has no variance claim. Godot process high-water
includes Core+Client. No concurrent build/test/benchmark during formal sampling.
The3/5/7GB budgets and60FPS target are unchanged; unavailable10K client or optional
LLM measurements must not become a fabricated full-configuration budget verdict.

## Evidence and compatibility

Tests compare instrumented and uninstrumented candidate/trace outputs and cover
actual query counting; benchmark smoke checks same-work equality and rejects
invalid CLI inputs. Existing tests/goldens are preserved. Reuse unchanged
simulation evidence across observational-only additions, documenting source
identities; do not mechanically repeat the full ten-year run for a query counter.
CHRON-034 owns seed0/1/42 regression goldens and hosted CI, and CHRON-036 only
consolidates verified results. Phase2 remains outside this instruction.

Implementation clarification: native memory tooling reuses the headless adapter's
benchmark fixture module (tools -> app adapter -> Core), never the reverse.
Neither Core nor Godot depends on an app. Tiny two-person test fixtures honor
Scheduler's existing `2 * live_entries + 64` compaction floor; formal >=100
scales retain the existing 8N diagnostic heap bound. No runtime rule changes.
The Rust leaf remained incomplete after one bounded rework, so the parent took
over its counters, strict CLI, probe, sampling and summary implementation.
