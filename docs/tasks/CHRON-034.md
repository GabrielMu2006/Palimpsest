# CHRON-034 — Deterministic Regression and CI

> **Status: Proposed — awaiting separate product-owner approval.**
> This Task is not authorized for implementation until the product owner explicitly approves this single Task.

## Objective
Establish a layered, deterministic regression suite and a CI pipeline: a seed corpus of worlds, a unit/integration/chaos-smoke layer, Linux Rust jobs and macOS Godot jobs, and a guard against using noisy CI numbers as a performance gate.

## Context
`MASTER_SPEC.md` §75 requires a Performance Test Suite and §76 a Chaos Simulation Test (§78 repository layout incl. `tests/simulation`, `tests/regression`, `tests/worlds`). ADR-0009 and PERFORMANCE.md forbid treating one noisy CI run as a performance gate. Phase 0 (CHRON-015/CHRON-017) built basic CI; Phase 1 needs deterministic regression to (a) prove the full 10-year loop (CHRON-032) at each milestone, (b) lock in the seed corpus so a bug cannot silently change history, and (c) keep the headless Core and the Godot client both covered. The Architecture Spike's "CI governance remains minimal" and "long-run stability unproven" risks specifically get addressed here.

## Scope
- Define a deterministic seed corpus (e.g., the CHRON-032 seeds plus a small fixed set) stored as world configs under `tests/worlds/` with exact expected invariants (final person count, produced-event range, no-violation assertions, deterministic report hash).
- Build a layered regression suite:
  - Unit: per-system tests (already covered per task) collected into a stable run surface.
  - Integration: cross-system contracts (kernel + action + needs + utility + movement + events + snapshot) run over short windows.
  - Chaos smoke: a shortened but real CHRON-032 10-year seed run (e.g., a fixed reduced-years seed) run as a CI gate so CI does not run the full 10-year CPU cost but still exercises the loop.
- Wire the Rust quality gates into Linux CI: rustfmt, Clippy with warnings denied, debug+release workspace tests, docs, dependency audit, and the regression/chaos-smoke layer. Keep the M5 reference-machine performance gates local (PERFORMANCE.md rule).
- Wire the Godot integration job into macOS CI: GDExtension discovery/init, scene smoke run, and a version of the CHRON-031 presentation smoke; mark the editor-exit crash path as a monitored risk per ADR-0010.
- Ensure CI does not assert performance thresholds from noisy numbers; CI runs only correctness/compile/support smoke. Any headless or rendered throughput is produced but labeled non-gating.
- Record the seed corpus, expected invariants, and exact CI commands. Configure stable Rust/Godot check names suitable for branch protection. Until GitHub permits private-repository rulesets on the current account, document the manual rule that `main` merges require those checks; do not claim server-side enforcement that does not exist.

## Out of Scope
- Using CI numbers as a performance gate (forbidden by PERFORMANCE.md and by this Task's guard).
- Relaxing or lowering the 3/5/7 GB caps, the 100-person budget, or the 60 FPS target.
- Implementing new gameplay systems.
- Replacing the M5 reference benchmark with CI; M5 local results remain authoritative.
- Deployment, release signing, artifact publishing.
- LLM, NLG, war, politics, religion, magic.

## Dependencies
- CHRON-032 complete (chaos runner + seed corpus used as the deterministic driver).
- CHRON-033 complete (benchmark harness methods/harness the CI smoke reuses only as compile/non-gating smoke).
- CHRON-015/CHRON-017 existing CI and CHRON-011/CHRON-031 Godot jobs.

## Files Modified / Allowed
- `.github/workflows/**` (new/updated CI jobs and required checks).
- `tests/simulation/**`, `tests/regression/**`, `tests/worlds/**` (seed corpus + expected invariants).
- `tools/**` and any helper scripts that drive CI/local checks.
- `docs/reports/CHRON-034_REGRESSION_CI.md` for the CI design/commands/gate policy.
- `docs/tasks/CHRON-034.md`.
- Optionally `crates/sim-core/**` for a regression harness/assertion helper, and a `MASTER_SPEC.md` hash guard (already exists) retained.
- No product doc change; no `MASTER_SPEC.md`, `docs/ARCHITECTURE.md`, or `docs/PERFORMANCE.md` edits without a Change Proposal.

## API Contract
- A deterministic regression entry, e.g. `run_regression(seed: WorldSeed, years) -> Result<WorldReport, RegressionError>` (reused by the chaos-smoke CI step), where the report includes the final-state hash and the invariant summary.
- A reproducibility contract: the same seed yields the same final-state hash; a mismatch between the corpus's expected hash and the produced hash is a CI failure.
- CI gates assert compile + tests + invariants only. Performance numbers are reported/logged but explicitly non-gating (a separate, documented variable).
- A `worlds/seed_*.json` config is paired with a reviewed expected-invariants/final-hash artifact. Golden results may be regenerated only by an explicitly approved behavior-changing Task, with the old/new diff and reason reviewed; tests may never rewrite goldens automatically.
- Godot CI smoke is a read-only presentation smoke; it never asserts simulation truth from the Scene Tree.

## Tests / Validation
- Seed determinism: each corpus seed reproduces its expected final-state hash across two runs.
- Regression harness: a deliberately broken world config fails the exact invariants and hash, proving the gate has teeth.
- Chaos smoke: the shortened seed run executes without error and within a CI time budget; the full 10-year seed (CHRON-032) is not run in CI but is reported as the reference-measured local gate.
- Layering: unit/integration/chaos-smoke each run and each can independently fail; no layer is skipped or weakened.
- CI syntax/validity: workflow check names are stable and locally equivalent commands pass. Server-side `main` enforcement is verified when GitHub account capability permits it; until then the limitation and manual merge rule are reported.
- Workspace gates: fmt, Clippy warnings denied, debug/release tests (local), docs, dependency audit, and a clean-checkout run.

## Benchmark
- No new M5 reference benchmark; this Task owns CI gating policy. It may report a CI *smoke* wall-clock and the chaos-smoke wall-clock for the record, but must state they are not a performance gate.
- The authoritative M5 scale sweep and 10-year timing remain in CHRON-032/CHRON-033 and MUST stay local per PERFORMANCE.md.

## Definition of Done
- A deterministic seed corpus (fixed worlds) with expected final-state hashes/invariants exists under `tests/worlds/`.
- A layered unit/integration/chaos-smoke suite runs under a clean checkout; each layer can fail independently.
- Linux Rust CI (compile/lint/tests/regression/chaos-smoke) and macOS Godot CI (GDExtension init + scene smoke) exist with stable names and are the intended required `main` checks; actual GitHub protection status is reported honestly.
- CI never gates on noisy performance numbers; any throughput is logged as non-gating.
- No test is deleted, skipped, or weakened; the editor-exit crash path remains a monitored (non-blocking) risk per ADR-0010.

## Required Completion Report
Report: change summary; commands run; the seed corpus list + expected invariants; the CI jobs added/updated and their exact triggers; the chaos-smoke wall-clock (as a CI record, not a gate); known limitations (e.g., CI smoke is not the full 10-year run; macOS Godot job platform constraints; editor-exit crash still monitored); and any blocker. Do not auto-start the next Task; each requires separate product-owner approval.
