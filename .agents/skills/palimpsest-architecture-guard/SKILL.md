---
name: palimpsest-architecture-guard
description: Guard Palimpsest architecture when changing public APIs, module boundaries, ECS identity, the Godot bridge, storage, history, LLM boundaries, or the Rule Engine. Use when a change can affect architecture or cross-module contracts.
---

# Palimpsest Architecture Guard

Read `MASTER_SPEC.md`, `AGENTS.md` when present, and relevant ADRs before reviewing or implementing the change. Verify these invariants:

- The Rust simulation core remains independently headless and authoritative.
- Godot remains presentation, rendering, and input; its Scene Tree and UI do not own simulation truth.
- Runtime ECS handles remain separate from persistent `EntityId` values.
- LLM functionality remains optional and cannot decide simulation truth.
- The Event Store remains structured, and history truth remains distinct from beliefs and historiography.
- Simulation LOD, identity, causality, and correctness are preserved.
- Cross-module public API changes are recorded in an ADR.

If a requested change conflicts with `MASTER_SPEC.md`, do not modify that specification or implement the conflicting portion. Create `docs/proposals/CP-XXXX.md` with Problem, Current Spec, Proposed Change, Reason, Impact, Migration, and Alternative, then stop the conflicting work for product-owner review.
