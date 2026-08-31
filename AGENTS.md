# Palimpsest Agent Instructions

These instructions apply to the entire repository. `MASTER_SPEC.md` at the repository root is the highest-authority, read-only product specification.

## Required Reading

Before work, read in full:

1. `MASTER_SPEC.md`
2. this file
3. `docs/ARCHITECTURE.md` when present
4. `docs/PERFORMANCE.md` when present
5. relevant ADRs and the current task specification

## Context Loading / Closed Task Routing

- After mandatory reading, use `docs/CURRENT_PROGRESS.md` as the active entry and `docs/TASK_INDEX.md` for task status/evidence routing. Read the active task and relevant current ADRs, not every historical plan linked transitively.
- Completed/superseded plan bodies and old reports are archival evidence, not a default reading queue. Load them only when changing their contract, investigating a regression, checking disputed evidence/approval, or when the user asks. An index summary cannot override Master Spec/ADRs or authorize implementation.
- Within the same task context, reuse a document already read in full if its content is unchanged; check changes before relying on it. Do not repeatedly dump the same full file merely because a subtask advances. New sessions or missing/compacted required context still follow Required Reading; this rule does not waive the four mandatory documents.
- Close a plan with a concise index entry: status/date, outcome, current ADRs, evidence/source identity, limitations and reopening trigger. Preserve historical files and links. Give subagents the relevant task packet (not full chat history), while retaining their mandatory project reading.

## Scope and Phase Boundary

- Execute bounded tasks in dependency order. Within an explicitly approved execution plan, continue to the next ready Task after verification without asking for the same approval again. Do not broaden scope or perform unrelated refactors.
- Phase 0 — Architecture Spike is complete and `docs/reports/ARCHITECTURE_SPIKE_V1.md` was confirmed by the product owner on 2026-08-29.
- Phase 1 planning, Task specifications, and ADR work are authorized.
- Phase 1 implementation requires approval of the Task or of an identified execution plan containing it. A request only to draft/review a plan is not implementation approval; an explicit instruction to follow/execute that plan is approval of its stated decisions and implementation steps.
- Phase 1 implementation scope is limited to World Grid, Terrain, Local Tile, Person Entity, Basic Movement, Time, Needs, Basic Utility AI, and the 100-NPC/10-year validation required by the Master Spec.
- Do not implement Phase 2+ systems, war, politics, religion, magic, historians, NLG, LLMs, Rule Editor, or a web client during Phase 1.

## Non-Negotiable Architecture

- The Rust Simulation Core is authoritative and runs fully headlessly without Godot.
- Godot owns presentation, rendering, input, and UI only. Scene Tree state is not simulation truth.
- Persistent identity is a stable domain `EntityId`; runtime ECS handles are never persisted.
- LLM functionality is optional and never decides simulation truth.
- Structured events, history truth, beliefs, and historiography remain distinct.
- Do not remove future LOD, Event Store, history, causality, or persistence boundaries for prototype convenience.

## Change Governance

- Never modify `MASTER_SPEC.md`.
- If a request conflicts with it, create `docs/proposals/CP-XXXX.md` using the proposal template, document the conflict and alternatives, then stop the conflicting implementation.
- Record cross-module public API, database, identity, ECS, serialization, Godot bridge, AI, history retention, NLG, or Rule IR decisions in an ADR.
- Do not delete, skip, weaken, or disable tests to make checks pass.
- Do not relax performance budgets without product-owner approval.
- Repository visibility: the product owner decided on 2026-08-30 that `GabrielMu2006/Palimpsest` must remain public. This supersedes earlier private-repository requirements; agents must not switch it to private without a new explicit product-owner decision.
- Keep `main` protected with strict required `rust-quality-and-smoke-benchmarks` and `godot-macos-integration` checks, administrator enforcement, and no force pushes or branch deletion. Verify live GitHub settings before claiming enforcement; documentation changes alone do not change remote settings.

## Task Contract

Every task specification must include Context, Scope, Out of Scope, Dependencies, Files Modified/Allowed, API Contract when applicable, Tests, Benchmark when applicable, and Definition of Done.

Before presenting an executable plan, follow `docs/EXECUTION_CONTRACT.md` and `docs/tasks/TEMPLATE.md`: inspect actual interfaces/callers and prerequisites, identify recommended decisions, include necessary tests/fixtures/benchmark tooling/platform/CI/documentation work, and map each DoD to evidence. Missing instrumentation is planned work, not an end-of-task waiver. Internal readiness/design checks are Codex's responsibility, not another product-owner approval gate.

Finish each task with the change summary, commands actually run, benchmark results or an explicit N/A, known limitations, and blockers. Continue only to ready Tasks already covered by the approved plan; do not enter a new phase or invent follow-on work.

## Approval Semantics — Product-Owner Clarification, 2026-08-30

- “按照你的计划来” / “按计划执行” means acceptance of the identified plan as a whole, including its stated recommended decisions, ADRs, file-scope additions, and planned actions. Record that acceptance once; do not request separate reconfirmation for each subtask, ADR status update, or agent dispatch already covered.
- Assess Luna fitness, file ownership, prerequisites, and tests internally before dispatch. This quality check is not an additional product-owner approval gate. Keep dispatch summaries short; independent verification remains mandatory.
- Resolve minor implementation details within the approved outcome autonomously. Stop only for a material unplanned change, an unresolved product choice not settled by the plan, a Master Spec conflict, or a real blocker that cannot safely be resolved in scope.
- At completion, summarize sensitive changes (for example visibility/protection changes, public API breaks, validation/rejection behavior, or removal of an audit mechanism) and remaining limitations. Reporting after execution does not authorize unrelated destructive actions or bypass higher-priority safety rules.
- This clarification supersedes older routine per-Task reconfirmation wording in project plans, task specifications, and skills. It does not retroactively approve every historical plan, waive tests/performance budgets, or authorize CHRON-027+ through the Phase 1 remediation plan.
