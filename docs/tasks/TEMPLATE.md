# TASK-ID — Title

> Status: Proposed / Approved / In progress / Verified complete / Blocked.
> Plan ID + revision: …; approval record: … (or not yet approved).
> Approval of the identified execution plan covers this Task. Do not request
> repeated per-Task/ADR/dispatch approval. A planning-only request is not approval.

Use [Execution Contract](../EXECUTION_CONTRACT.md). Remove instructional
placeholders before calling the Task ready; keep the document proportional to risk.

## Context

Current implementation and evidence; why this one outcome is needed. Name the
actual upstream APIs/reports; distinguish planned from existing capabilities.

## Objective

One observable result; no open-ended “continue improving”.

## Scope

Deliverables including necessary adapters, error handling, tests and measurement tools.

## Out of Scope

Explicit exclusions and end boundary; no implicit next Phase or remote operation.

## Dependencies

Task IDs + required artifacts/contracts. At execution start, parent verifies
these against the worktree; dependency approval alone is not completion.

## Decisions / Execution Readiness

- Accepted constraints / ADRs:
- Recommended choices accepted with this plan (including compatibility impact):
- Internal details parent may settle without new approval:
- Any genuinely unresolved product choice (resolve before claiming Ready):
- First step: record exact signatures/fixtures/tool commands, then implement.

## Files Modified / Allowed

Separate implementation files, directly affected callers, tests/fixtures,
benchmarks/tools, manifests/CI, and documentation/report/ADR paths. Permit
bounded same-module helper/test additions where appropriate; a directory
allowance is not permission for unrelated refactoring. List external mutations
separately, or state “local changes only; no commit/push/merge/settings change”.

## API Contract

Inputs/outputs, ownership, state transitions, invalid native/serialized inputs,
error/partial-commit semantics, invariants and compatibility. Name the ADR for
cross-module changes. N/A is valid for documentation-only tasks.

## Execution Steps / Agent Ownership

Parent prepares contract → bounded implementation → negative/integration tests
→ measurement → independent verification → report → next approved Ready Task.
If delegating, state Luna fitness, disjoint files and parent-owned interfaces.
Do not delegate unresolved product/architecture choices or waive review.

## Tests

Exact existing commands + expected results; mark commands to be created.
Include failure/recovery tests and full affected-caller compilation. Distinguish
local checks, hosted CI and manual review. Never weaken tests to obtain a pass.

## Benchmark

N/A with reason, or: scenario/seed/count/duration, metric/unit, interval/control,
instrument and adapter readiness, build/hardware, warm-up/sample count, raw
output path, correctness assertions, gate/diagnostic status and limits.

## Risks / Stop Conditions

Task-specific risks, in-scope recovery, and real scope/authority blockers.
Ordinary bug fixes, file-list refinements and planned ADR work are not reapproval.

## Definition of Done

Observable conditions, each mapped to a test/measurement/artifact above.
Required missing evidence means incomplete, not “done with N/A”.

## Required Completion Report

Actual files/commands/results, DoD evidence, benchmark or justified N/A, sensitive
changes and limitations. Continue to the next Ready Task already in the accepted
plan; stop at its terminal boundary, never automatically enter the next Phase.
