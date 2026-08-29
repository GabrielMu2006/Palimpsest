# Palimpsest Agent Instructions

These instructions apply to the entire repository. `MASTER_SPEC.md` at the repository root is the highest-authority, read-only product specification.

## Required Reading

Before work, read in full:

1. `MASTER_SPEC.md`
2. this file
3. `docs/ARCHITECTURE.md` when present
4. `docs/PERFORMANCE.md` when present
5. relevant ADRs and the current task specification

## Scope and Phase Boundary

- Execute one bounded task at a time. Do not broaden scope or perform unrelated refactors.
- Current work is Phase 0 — Architecture Spike only.
- Do not implement NPC AI, war, politics, religion, magic, historians, NLG, LLMs, Rule Editor, or a web client during Phase 0.
- Do not enter Phase 1 before `docs/reports/ARCHITECTURE_SPIKE_V1.md` is confirmed by the product owner.

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

## Task Contract

Every task specification must include Context, Scope, Out of Scope, Dependencies, Files Modified/Allowed, API Contract when applicable, Tests, Benchmark when applicable, and Definition of Done.

Finish each task with the change summary, commands actually run, benchmark results or an explicit N/A, known limitations, and blockers. Do not automatically start the next task.

