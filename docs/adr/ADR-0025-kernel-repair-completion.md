# ADR-0025 — Complete kernel boundary and measurement repair

- Status: Accepted within the owner's explicit 2026-08-31 request to plan and implement the reviewed fixes with subagents.
- Scope: P1-KERNEL-REPAIR-V2; supplements ADR-0024, never the Master Spec.

## Context

Independent review found eight failing contract probes despite 97 passing existing tests, plus incomplete measurement protocol. This records the narrow public and observation contracts before edits.

## Decision

- Action start/cancel preflight every fallible Needs/time/token/event requirement before mutation. Track a separate per-person last successful action commit instant, updated at movement/arrival/retry/completion/check/start/cancel; this is not a second Needs baseline. Reject earlier requests. No whole-world clone or rollback framework.
- Keep `WorldKernel::metrics() -> KernelMetrics`, adding `state: KernelState` and `failed_at: Option<SimInstant>`. Cache only bounded last-complete action queue/count diagnostics, refresh on successful start/round, and expose that cache rather than faulted live execution state. Committed event counts remain separately available. `health.last_complete == now` in every state.
- Change `WorldKernel::next_due` to `Result<Option<SimInstant>, KernelReadError>` and `sites` to `Result<&ActivitySites, KernelReadError>`; Faulted returns an explicit error. ActivitySites includes mutable-in-simulation WorkCounter, so it is not a pure static exemption. No new bypass. Projection errors also become typed read errors rather than silently returning stale Needs; render builder propagates them.
- `KernelAdvance.events` counts all successfully generated events for this call, including records dropped by bounded retention, matching cumulative metrics deltas. Event schema, FIFO and FNV encoding unchanged.
- Independently deserializable terrain/person DTOs use validated private wire forms. Root decode/builder share their validators; Moving Idle is invalid. Keep transient schema 2 and no persistence migration.
- Fix measurement contracts without touching native RSS: strict CLI, exact upper median, full cumulative rounds and measured-boundary max queue, raw sample consistency, all retained objects alive at second observation. Record source identity. Kernel returns to planned first-walkable colocated spawn; previous V1 BFS dataset is non-comparable and retained as historical, not optimization evidence.
- Read current task/ADR from a concise progress/index entry; finished plan bodies are archival evidence and loaded only when relevant. Preserve mandatory Master/AGENTS/Architecture/Performance reading. This is retrieval guidance, not removal of test or audit requirements and not a token saving guarantee.

## Alternatives / Consequences

Rejected: metrics returning unmarked live partial state; silently returning None/old Needs; cloning the world; dropping old evidence; marking pilot or aggregates as raw; weakening required verification.

The two read signatures are local Phase1 API tightening; all current sim-core callers are adapted together. Metrics now advertise health. No Godot/worker/storage/identity/AI rules change. Formal measurements must be regenerated on final runtime/tool sources; 10-year validation remains CHRON-032.

## Verification

See the six tasks and precise DoD in [repair V2](../tasks/P1_KERNEL_REPAIR_V2.md). Parent owns independent regression, artifact/source checks and final gate. Closed-task index is an execution aid, not authority over ADR or Master Spec.
