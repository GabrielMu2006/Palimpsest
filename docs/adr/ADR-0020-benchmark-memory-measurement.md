# ADR-0020: Benchmark-Only Peak RSS Measurement

- Status: Accepted within the owner-approved REM-008A tooling extension.
- Date: 2026-08-30
- Extends ADR-0001 for one private native instrumentation boundary only.

## Context and Decision

Use an outward-only `tools/bench-memory` workspace binary. It imports domain
libraries and existing example source modules; no simulation library depends
on this tool and no domain API or wire format changes. Additive example
adapters reuse fixtures and checks; original timing paths remain unchanged.

On macOS, `task_info(MACH_TASK_BASIC_INFO)` for the current task provides
`resident_size` and `resident_size_max` in bytes. Apple's implementation reads
the physical-memory ledger's balance and lifetime maximum respectively:
[XNU task.c](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/task.c).
These are RSS, not allocator allocation totals or physical footprint. Native
reads capture the kernel high-water counter, not a periodically sampled peak.

Each cold sample is a new `Command` subprocess (exec, not a forked running ECS).
Let B be current RSS at the interval start, H0 its lifetime high-water counter,
E current RSS at interval end, and H1 its ending high-water counter.

- If H0 == B, or H1 > H0, the interval maximum is provably H1. Report H1 - B.
- Otherwise the earlier peak may mask this interval. Report null/ambiguous;
  do not subtract H0 or pretend an endpoint delta is an interval peak.
- Reject inconsistent values (H0 < B, H1 < E, or H1 < H0).

Record two intervals: cold case start through operation/result check (including
fixture creation and first-use code/allocator costs), and prepared fixtures
through operation/result check. Original warm timing numbers and old retained
deltas are separate series. The cold series must be provable for every sample;
an ambiguous cold interval fails the run, with no silent retry/sample removal.
Prepared-only intervals may explicitly be unavailable if setup's earlier peak
masks them. No claim of per-object heap size follows from either RSS series.

## Native Safety and Dependencies

The binary alone uses already-locked `libc` bindings on macOS. Its lint policy
matches workspace all/pedantic Clippy, denies unsafe by default, and permits
unsafe only on the private, documented Mach-read and diagnostic-mapping
functions. The measurement call uses the
current task, ABI-provided struct/count/flavor, writable initialized storage,
checks the return code/count, and reads copied fields only after success.
There is no foreign pointer input, process attachment, privilege change,
injection, signal handler, sampling thread, allocator replacement or game-loop
callback. All production crates retain inherited `unsafe_code = forbid`.
This is the narrow exception expressly permitted by ADR-0001, not a relaxation
of simulation safety or performance budgets. No new third-party version is
introduced; no domain dependency manifest changes.

The second native function is used only by diagnostic probe selectors: it
maps, touches and unmaps a fixed 64 MiB anonymous private region. It accepts no
address, fd or size, checks mmap/munmap errors and exposes no pointer. This
avoids malloc/free caching, which can leave freed Rust Vec pages resident and
would not actually test a transient resident peak. It never changes the
allocator of the measured simulation workloads.

## Verification and Consequences

Algorithm tests cover both proof conditions and contamination. Real child
probes touch/free 64 MiB, proving the lifetime counter retains a transient peak
that ending RSS misses. A prior-large-peak probe verifies prepared-operation
rejection. Output stores both endpoints/high-water values, PID, checksums and
sample index. macOS is the measured platform; other platforms report unsupported.

RSS has OS page granularity, includes first-touch shared code and allocator
effects, and is subject to normal OS memory pressure. Measurements are exact
with respect to the kernel's accounting/proof, not byte-exact object sizes.
The cold series intentionally includes setup; prepared-only numbers cannot be
inferred by subtracting independently measured cold fixtures.

## Alternatives

- `ps` polling: can miss short-lived peaks; retain as historical endpoints only.
- Whole-command `time -l`: cannot separate multiple cases or setup.
- Difference of lifetime peaks: measures new high-water growth, not peak minus
  current baseline, and can report false zero after an earlier large workload.
- Allocator counters/physical footprint: useful other metrics, not RSS.
- Fork/reset/injection: unnecessary process/allocator/permission complexity.
- Change production crates or add a general monitoring system: out of scope.
