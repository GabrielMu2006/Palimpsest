# CHRON-018 Workspace Boundaries — REM-002 Completion Report

## Context

REM-002 closes the conditional review of CHRON-018. The review corrected the
meaning of dependency allow-sets, clarified the outer `godot-bridge` adapter
boundary, replaced name-substring architecture inference with exact Cargo
metadata/tree review, and removed the custom audit integration after recording
equivalent present-state evidence.

## Scope

Documentation clarification, comments-only manifest clarification, removal of
the custom dependency audit integration, and verification of the existing
workspace graph. Existing CHRON-019..026 implementations and dependencies are
preserved.

## Out of Scope

No crate-boundary changes, domain behavior, API changes, CI changes, scripts,
new dependencies, Godot code, LLM integration, or dependency removal beyond
the approved audit integration.

## Dependencies

ADR-0001 and ADR-0017; Cargo metadata and normal dependency trees; the
product-owner-approved REM-002 execution plan.

## Files Modified / Allowed

- `docs/tasks/CHRON-018.md`
- `docs/adr/ADR-0017-phase-1-crate-boundaries.md`
- `docs/ARCHITECTURE.md`
- `crates/sim-ai/tests/dependency_direction.rs` (removed)
- `crates/sim-ai/Cargo.toml` (comments only)
- `crates/sim-world/Cargo.toml` (comments only)
- This report

## API Contract

No public API changed. Allow-sets are permitted normal dependency sets, not
mandatory edges. Current direct normal sets are:

- `palimpsest-sim-world`: `{serde}`; allowed
  `{palimpsest-sim-entity, palimpsest-sim-time, serde}`.
- `palimpsest-sim-ai`: `{palimpsest-sim-time, palimpsest-sim-world, serde}`;
  allowed `{palimpsest-sim-entity, palimpsest-sim-time,
  palimpsest-sim-world, serde}`.

The outer adapter rule is that simulation/domain crates do not depend outward
on `palimpsest-godot-bridge`; the bridge may depend inward on simulation-facing
crates. LLM/Godot constraints are reviewed from exact metadata dependency
names, not `contains`/`starts_with` heuristics.

## Tests and Evidence

Before removal: `cargo test -p palimpsest-sim-ai --test dependency_direction`
passed 4/4. `cargo metadata --no-deps --format-version 1 --offline` confirmed
workspace membership and exact package dependency declarations. The
normal-tree transitive closure (not a claim about direct edges) contains
`palimpsest-sim-time`/`palimpsest-sim-world` as applicable and
`serde 1.0.229`, `serde_core 1.0.229`, `serde_derive 1.0.229`,
`proc-macro2 1.0.107`, `quote 1.0.47`, `syn 3.0.4`, and
`unicode-ident 1.0.24`. The exact nesting and edge structure is provided by
the commands below; `unicode-ident` is reached through the procedural-macro
dependency chain, not directly from `serde_derive`.

```text
cargo tree -p palimpsest-sim-world --edges normal --offline
cargo tree -p palimpsest-sim-ai --edges normal --offline
```

Post-change verification also passed:

- `cargo metadata --no-deps --format-version 1 --offline`
- `cargo tree -p palimpsest-sim-world --edges normal --offline`
- `cargo tree -p palimpsest-sim-ai --edges normal --offline`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features` (all listed tests
  passed: sim-ai 44, sim-core 12, sim-entity 9, sim-events 3,
  sim-scheduler 8, sim-storage 4, sim-time 9, sim-world 58)
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`
- `git diff --check`

This review uses manual/agent metadata/tree evidence at the workspace CI/lint
gate and whenever dependencies change; it does not claim automatic future
enforcement.

## Benchmark

N/A — this change alters documentation and audit mechanism only; it adds no
runtime behavior or performance claim.

## Independent Parent Acceptance — 2026-08-30

The Codex parent inspected the complete diff and accepted REM-002 after one
bounded correction: the extra allow-set restrictions apply to sim-world and
sim-ai, not every simulation crate; dependency evidence distinguishes the
transitive closure from direct edges; Phase 0 headless-runner ECS usage is
not mislabeled a new boundary violation.

The parent independently reran normal dependency metadata and both Cargo
trees, `cargo fmt --all -- --check`, `git diff --check`, and
`cargo test --workspace --all-targets --all-features`. All 147 remaining
unit/integration tests passed (zero failed or ignored). This count is the
REM-002 checkpoint before subsequent remediation tests are added.

The four custom dependency-audit tests were deliberately removed under the
approved audit-mechanism change, after passing 4/4. They were not removed to
make a failing gate green. No domain tests were removed; future graph
enforcement now depends on explicit metadata/tree review rather than those
four automated checks. Both manifest diffs contain comments only; the
dependency declarations and Cargo.lock are unchanged.

## Definition of Done

- All six REM-002 clarifications are traceable in the task, ADR, and
  architecture documentation.
- Current legitimate dependencies remain unchanged.
- The substring-based custom audit integration is removed after the 4/4
  pre-removal evidence.
- Domain source, behavior, tests, APIs, and golden expectations are unchanged.
- The historical empty-skeleton state is distinguished from current populated
  crates.

## Limitations and Blockers

The exact graph review is more precise than the removed substring audit, but
manual/agent review does not automatically enforce future dependency changes.
No blocker is known within REM-002. Phase 1 runtime `bevy_ecs` remains in the
composition/runtime boundary; Phase 0 headless runner usage remains legal as
documented by the existing architecture.
