# ADR-0027: Phase 1 Headless 10-Year Chaos Runner Contract (CHRON-032)

- Status: Accepted — covered by the product owner's explicit CHRON-032
  implementation instruction on 2026-08-31
- Date: 2026-08-31
- Decision owners: Product owner
- Task: CHRON-032 in `docs/tasks/CHRON-032.md`; kernel/DTO/worker semantics per
  ADR-0015, ADR-0021–0026
- Extends: ADR-0002, ADR-0004, ADR-0013, ADR-0022, ADR-0024, ADR-0025; does not
  supersede the Master Spec

## Context

The Master Spec makes "continuous 10 simulated years without crashing" the
Phase 1 Definition of Done and requires a chaos test proving no extinction, no
unbounded growth, no NaN, no infinite loop, no dangling references, no
unbounded queue, and no obvious leak. CHRON-032 is the headless vehicle for that
proof. The kernel (CHRON-028, ADR-0022, repaired by ADR-0024/0025) is the
authoritative owner of time/ordering, so the runner is an outer driver only: it
never mutates world state directly and never teleports a person.

## Decision

### 1. Calendar and defaults (D4)

- `SECONDS_PER_DAY = 86_400`, `DAYS_PER_YEAR = 365`, so
  `SECONDS_PER_YEAR = 31_536_000` and **10 years = `315_360_000`** seconds from
  the epoch. This is the authoritative Phase 1 horizon; the "year" is not a new
  calendar lore, and no reduced-duration substitute is accepted.
- `ChaosConfig { seed: u64, person_count: usize, years: u64, sim_seconds_per_year: i64 }`
  defaults `person_count = 100`, `years = 10`, `sim_seconds_per_year = 31_536_000`.
  The run target is `years * sim_seconds_per_year` seconds.
- The kernel uses `KernelConfig::default()` (work budget 1,024 due-instant
  rounds, event buffer 4,096) unless overridden; D1/D2 action/duration rates are
  the already-implemented ADR-0013/0021 values and are **not changed**.

### 2. Deterministic 100-person fixture

Spawn positions are computed from the fixed `seed` (42) map, not hard-coded, and
are **proven reachable without teleport**:

1. `WorldMap::generate(seed, default)` and `ActivitySites::place_defaults(&map)`.
2. Flood-fill the walkable grid to form connected components (row-major scan).
3. Drop every component that does not contain ≥1 Meal **and** ≥1 Rest **and**
   ≥1 Work site.
4. Collect the surviving components' walkable cells in row-major order and take
   the first `person_count` distinct cells as spawn coordinates.

Every spawned person therefore has a real path, via existing pathfinding, to a
Meal, a Rest, and a Work site, so the Phase 1 action/needs loop executes real
movement, eating, sleeping, and working. If no component contains all three
kinds, `run_chaos` returns a typed error rather than manufacturing a selection or
placing persons at a reduced site set.

### 3. `run_chaos` API (lives in `palimpsest-sim-core`, not Godot)

```text
pub fn run_chaos(config: &ChaosConfig) -> Result<ChaosReport, ChaosError>
pub fn run_chaos_with_watch(config: &ChaosConfig, watchdog: &mut dyn FnMut()) -> Result<ChaosReport, ChaosError>
```

- `run_chaos` is the public headless entry; the watchdog variant is the
  externally-observed liveness guard (the bin maps wall time and calls
  `run_chaos_with_watch`).

### 4. Invariants and detectors (the instrument lives in the Core)

The runner checks, at **every simulated-day checkpoint** and at the final
boundary, and returns a typed `ChaosError` (non-zero bin exit) on the first
violation:

- `NonFinite` — a person's hunger/fatigue raw value outside documented bounds
  `[0, NEED_MAX = 100_000]`, or a reported quantity outside its declared
  interval. All Phase 1 quantities are integers, so strict NaN/Inf is
  structurally impossible and the check is a bounds check on the integers that
  carry the same meaning.
- `QueueGrowth` — `scheduler_queue_depth > 2 * person_count` (D2: ≤2 live
  schedule items per person) or `queue_nodes > 8 * person_count` (live + lazily
  invalidated nodes, bounded by compaction).
- `DanglingReference` — a drained outcome event whose actor or target
  `EntityId` is not a live population id / known site id, or which is otherwise
  unresolvable.
- `Invariant` — any other documented rule violated: monotone clock, committed
  boundary ≤ target, buffer `total = delivered + buffered + rotated`, per-day
  population count preserved.
- `NonTerminating` / `Watchdog` — the outer liveness guard. Per advancing
  `advance_to` call is allowed to yield (`reached_target == false`), but the
  committed instant must strictly increase; if `committed_to` stalls for
  `MAX_STALLED_CALLS` calls, `NonTerminating` fires; the wall-clock watchdog in
  the bin reports `Watchdog` on a hard liveness miss. The runner never pretends
  to recover a panic or an infinite loop.

All bounds are deterministic assertions against fixed, documented thresholds;
they are not ad hoc warnings, and they are justified above.

### 5. Canonical truth hash and digests (deterministic across runs)

- **Truth hash**: an FNV-1a-64 stream over a canonical serialization of config
  (seed/count/horizon), final `SimInstant`, per-person `(id, location, action,
  action_target, state, needs)` in ascending `EntityId` order, the sorted site
  set, and kernel counters (`rounds_total`, `transitions_total`,
  `decisions_total`, `events_total`, `events_digest`, `scheduler_queue_depth`).
  **It excludes** wall-clock time, RSS, thread identity, pointers, and ECS
  handles (ADR-0002). Same `seed`/config/input ⇒ same hash this run and next.
- **Event/digest accounting**: the kernel already exposes a cumulative
  deterministic event digest and `total = delivered + buffered + rotated`
  (ADR-0024/0025). The runner additionally folds a **per-person action
  completion digest** while draining events (`action.completed` ⇒
  `(person_id, action_kind)`), which is reported and compared across runs.
- Different fixed seed ⇒ different truth hash (asserted with a second seed).

### 6. Report fields

`ChaosReport` is `serde`-serializable JSON and carries: config echo, final
`SimInstant`, `person_count`, per-simulated-day samples (population, per-action
distribution, queue depth, needs finite/bounds ok), aggregate and per-person
action-completion counts (movement phase reported separately from top-level
selection — a completed Eat/Sleep/Work necessarily required a real completed
movement phase because no teleport exists), idle observation count, events total,
`events_per_wall_second`, `sim_seconds_per_wall_second`, peak process RSS (from
the memory-tool wrapper; checkpoint RSS is labelled a trend sample, not an exact
peak), truth hash, event digests, and the (empty-on-success) violated invariants
list. Death statistics are `NotApplicable` (Phase 1 has no death system).

**Completion gate (DoD).** Every person completes an Eat, a Sleep, a Work, and a
real movement phase (each activity's reach phase; no teleport). A top-level
standalone `Move` is **not** required — persons reach an activity site through
the activity's own movement phase, so counting a selected or manufactured `Move`
would be exactly the forbidden shortcut. Idle is *reported*, not gated: under
the ADR-0018 default weights (Work 2300 vs Idle −50) a fully-reachable
Work/Meal/Rest fixture never selects Idle, so this run records Idle as genuinely
unobserved while the Idle instrument is separately proven by a unit test
(empty-site fixture leaves Idle as the only viable action).

## Consequences

- The Core proves the Phase 1 gate headlessly; the bin is a thin parse/report
  wrapper, and Godot is uninvolved.
- The run is in-memory: no save/load. Event Store durability and database
  consistency are `NotApplicable` (deliberately out of scope, not omitted).
- The fixture and detectors are deterministic and reusable by CHRON-033/034.
- Wall-clock duration and RSS are measurement fields, excluded from equality.

## Rejected / Deferred Alternatives

- Reduced-duration substitute for the 10-year gate: rejected (D4).
- Manufacturing selections or counting a selected action as completed: rejected.
- Building a new economy/ageing/death system for this gate: rejected (out of
  Phase 1 scope).
- Persisting the run or asserting Event Store durability: deferred (CHRON-034+).
- Placing the instrument in Godot or the bin: rejected; it must live in Core.

## Task Completion / Acceptance Gate

- Files: `crates/sim-core/src/chaos.rs`, `crates/sim-core/src/lib.rs` exports,
  `apps/headless-runner/src/bin/chaos_runner.rs`, tests, report, and the
  documentation sync listed in CHRON-032.
- Tests: detector predicate unit tests, determinism (same seed **and** different
  seed), short-horizon completion + population preservation + finite/bounded
  checks, and the three-run benchmark on M5.
- DoD: per `docs/tasks/CHRON-032.md`; the 10-year gate is the hard
  correctness/no-crash gate, not a throughput claim (CHRON-033/036 hold the
  budget claims).
