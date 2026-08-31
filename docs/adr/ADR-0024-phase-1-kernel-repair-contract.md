# ADR-0024: Phase 1 Kernel Repair Contract

> Current supplement: [ADR-0025](ADR-0025-kernel-repair-completion.md).
> Historical decisions below are retained; use the supplement for V2 boundary repairs.

- Status: Accepted — approved with the P1-KERNEL-REPAIR / 2026-08-31-r1
  execution approval
- Date: 2026-08-31
- Decision owners: Product owner
- Task: `docs/tasks/P1_KERNEL_REPAIR_V1.md`, findings F01–F06 and gaps G01–G03
- Extends / locally supersedes: ADR-0021 (action execution), ADR-0022 (kernel),
  ADR-0023 (render DTO). This ADR augments those decisions; it does not rewrite
  their recorded history.
- Does not supersede: the Master Spec, ADR-0002/0003/0004/0009/0013/0014/0018/0019.

## Context

A 2026-08-31 review of the uncommitted CHRON-027/028/029 implementation
confirmed seven behavioral defects (F01–F06) and three delivery gaps (G01–G03)
against the accepted contracts. The defects were reproduced against the exact
reviewed source identity (recorded in the plan §1). This ADR records the
repair decisions D1–D6 in writing before the first source change, per the
plan; the decisions are the acceptance surface and are approved as a whole.

## Decision

### D1 — Rejections are side-effect-free; fatal execution errors stop

`start` and `cancel` pre-validate identity, current action, target/path, time,
and the follow-up due/token/EventId budget **before committing** any mutation.
A rejected call must not find a cancelled retry token, mutated Needs, a removed
action, a consumed `EventId`, or a changed queue. Validation uses private
prepared-transition/checked values and touches only the current person's data
(no world copy, no general transaction/rollback layer).

Where multiple follow-up tokens must be jointly pre-validated, a single narrow
Scheduler public extension is allowed:

```rust
impl<T> Scheduler<T> {
    pub fn check_schedule_capacity(&self, count: usize) -> Result<(), SchedulerError>;
}
```

It checks only the token/order remaining space and does not mutate the queue.
During a single-threaded call there is no other inserter, so the pre-validated
count must cover exactly the same requests the commit will make. The Scheduler
itself, its FIFO ordering, token format, and the ADR-0004 contract are
unchanged; no concurrent reservation system is added.

Internal due-work fatal failures follow D3: an unfinished boundary is never
reported as success, but already-committed history is not rolled back. The
expected blocked/failed activity recovery remains Idle + a positive retry delay
and is not promoted to a world-fatal error.

### D2 — One decision per (person, instant)

The raw due work and outcome events still execute in the original due-time/FIFO
order (ADR-0004 unchanged). After one full due instant, **decision requests are
merged by `(EntityId, SimInstant)`**: one fresh selection runs when a
`Completed`/`Retry` request is present; a `CriticalBoundary` request alone
compares and interrupts only when it elects a different `(kind, target)`. A
person that completed at that instant is not also reported interrupted. The
selection reads the final Needs of that instant, including the relief applied
by a completed Eat/Sleep. Merge order across persons preserves the first
occurrence order (no HashMap/identity reordering).

The merge is implemented once in a location shared by the kernel driver and the
`run_until` reference driver (a single batch helper). It is not a
Kernel-only swallow of `AlreadyExecuting`, and it does not adjust the 44,999s
fixture or any default duration to dodge the collision.

### D3 — Minimal lifecycle and full-boundary visibility

The kernel has `Setup / Running / Faulted` states; a new kernel is `Setup` at
epoch. `spawn_person` is allowed only in `Setup` at epoch. `start_world(at)` is
allowed only in `Setup` with `at == now() == EPOCH`, after which the kernel is
`Running`; a second call, an earlier/later target, or a spawn after start is a
typed error with no person/allocator/queue/clock mutation. A non-empty `Setup`
world that has not been started rejects a forward advance with a
`NotStarted`-style error; equal-target is a side-effect-free no-op; an empty
`Setup` world may advance directly to the target and become `Running`
(preserving the existing empty-world behavior), after which `start`/`spawn` are
rejected.

Recoverable input rejection never sets `Faulted`. A real execution/decision/ID-
exhaustion error records `last_complete`, `failed_at`, and a typed cause and
sets `Faulted`; the kernel then rejects further mutation. Phase 1 re-creates
the world to re-run; no "clear the error and continue" API and no silent
retry/rollback. `now()` always reports the last complete boundary; counts,
latest traces, and drained events never count a failed instant's partial work.
Dynamic read entries check the full boundary: `Faulted` reads fail, while the
static map, fixed population count, health-with-failure-marker, and prior
complete-boundary events remain diagnostics. No unchecked public live-state
bypass remains beside `RenderSnapshot`.

The recommended public read API becomes:

```rust
pub fn person(&self, id: EntityId) -> Result<Option<KernelPersonView>, KernelReadError>;
pub fn persons(&self) -> Result<Vec<KernelPersonView>, KernelReadError>;
pub fn latest_trace(&self, id: EntityId) -> Result<Option<&DecisionTrace>, KernelReadError>;
pub fn snapshot(&self) -> RenderSnapshot; // or the fallible builder below
RenderSnapshot::from_kernel(&WorldKernel) -> Result<RenderSnapshot, RenderError>;
```

The due-instant round budget unit from ADR-0022 is retained; the documentation
no longer equates 1,024 rounds with 1,024 items or promises a real-time
response bound (a single round's cost scales with population).
`advance_to(_, 0)` returns `InvalidBudget`; `KernelConfig::new` becomes
`Result` and rejects a zero default budget or zero event capacity, while
`Default` stays valid.

### D4 — One lazily-updated Needs baseline

`ActionRuntime.last_needs_at` remains the authoritative materialization
baseline. Read views compute `stored_needs.advance(now - last_needs_at)`
through a read-only helper without re-growing, writing back, or scheduling work.
A newly spawned, action-free person starts at the legal epoch baseline. The
decision driver also selects against the Needs projected to the request
instant, avoiding "select on old values, then materialize on start". Needs
growth rates (1/2), saturation, relief, utility weights, and default durations
are unchanged; `sim-ai` needs/utility are not modified to accommodate the view.

### D5 — Count and digest before rotation

Every successfully committed high-level outcome `EventRecord` is validated,
counted, and folded into an ordered stream digest **before** entering the
4,096-record diagnostic buffer. The existing two-level buffering (action +
kernel) is kept; `ActionRuntime` accumulates a total/digest, and the kernel
reads the cumulative totals and the upstream-drop delta after a full instant,
then processes its own retained buffer. `events_rotated` counts actually lost
records across both levels, each record once; the total is never inferred from
the surviving `Vec` length.

The digest is a versioned, non-cryptographic FNV-1a-64 stream over each event's
canonical field order JSON UTF-8: fold a little-endian `u64` byte length first,
then the body; metadata keys are stably sorted. Offset basis
`14695981039346656037`; prime `1099511628211`; the multiply is explicit
wrapping. This is a deterministic diagnostic, not tamper-proofing or a
collision guarantee; other real counters must not silently wrap. `total` is:
`delivered_to_consumer + currently_retained + actually_rotated`. The EventRecord
schema, persistence, and retention policy are unchanged. Digests/totals are not
exposed as a `Faulted` instant's complete-kernel statistics.

### D6 — Complete, verifiable Render DTO, explicit schema bump

`RENDER_SCHEMA_VERSION` moves **1 → 2** and schema 1 is rejected; Phase 1 makes
no compatibility promise for old diagnostic JSON and adds no migration or
SQLite/save-snapshot change. New fields:

- Static `ActivitySiteRender { coord: LocalCoord, kind: SiteKind }` batch read
  from the kernel's real `ActivitySites`, sorted by `(y, x)` with unique
  coordinates; sites of the same kind enumerated via the existing `sites_of`.
  No fabricated `EntityId`, no inventory/economy/dynamic-site /new-map-truth.
- `PersonRender` gains the snapshot instant's read-only `Needs` (kernel
  projection), retaining stable `EntityId`, `tile`, `action`/`target`/`state`.
- `RenderMetrics` gains observable `live_actions`, and `rounds_total`/
  `transitions_total`/`decisions_total` (definitions from the kernel). Idle
  waits are not mislabelled "not executing"; no unmeasured FPS/RSS zero.

The full builder and the diagnostic decode share one structural validation:
`width == height == 128` and `cells.len() == 16_384` (dimensions checked before
any huge-dimension product), non-zero/unique/ascending person identities,
metric-count match, existing `LocalCoord`/`Needs` validation, and new site
bound/uniqueness/ordering/walkability checks. Action/state/target must agree:
`Idle`/`Idle`/`None`; `Moving` matches its kind and has a target;
`Eating`/`Sleeping`/`Working` correspond to `Eat`/`Sleep`/`Work` with a target.
Illegal wire is rejected, never silently repaired. Every publicly
independently-deserializable batch/person DTO checks its own invariants;
cross-batch correlation is validated by the root DTO. No "restore world / run
action from JSON" path is added.

## Consequences

- CHRON-030/031 remain out of scope; the kernel/DTO adapters are mechanical.
- The 100-NPC/10-year gate remains CHRON-032; the annual benchmark recorded here
  is throughput evidence, not a Phase 1 completion claim.
- Existing ADR-0021/0022/0023 decisions stand except where this ADR locally
  supersedes them (rejection atomicity, request merge, lifecycle, Needs view,
  event accounting, schema 2).

## Task Completion / Acceptance Gate

- Dependencies: CHRON-027/028/029 current implementations, accepted ADRs, and
  the P1-KERNEL-REPAIR execution approval recorded 2026-08-31.
- Files: this ADR plus the KFIX-001..008 allowed implementation/report surface.
- Tests and benchmark: per `docs/tasks/P1_KERNEL_REPAIR_V1.md`; the regression
  fixture is `crates/sim-core/tests/kernel_repair.rs` and measurements follow
  the plan's 2+10 timing and 3-cold RSS protocol.
- DoD: the findings F01–F06 and gaps G01–G03 are closed with evidence, the
  workspace gates pass, and no forbidden file/behavior is changed.
