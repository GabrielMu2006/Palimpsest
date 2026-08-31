# CHRON-034 — Deterministic regression and CI

Status: preparation during CHRON-033 sampling; local and hosted final gates pending.
Owner authorization: current request to complete033–036, P1-REMAINING/r1.

## Contract

The corpus uses seeds0/1/42, 100 persons and86400 actual simulation seconds.
Each config is paired with the complete deterministic schema2 chaos report,
including invariants, per-person work, daily state, queue observations and hash.
`apps/headless-runner/tests/seed_regression.rs` runs each twice and compares to the
reviewed expectation. A corrupted expected hash is deliberately rejected without
changing the file. Tests never generate or rewrite expectations. A future behavior
change needs its own approved scope and reviewed old/new fixture diff.

Unit tests remain system-local. Core integration tests cover committed actions,
needs, movement, events, workers and snapshots. The registered corpus integration
test is the real chaos-smoke layer; it is not the 3650-day reference validation.
Debug and release suites exercise these tests, and doctests are separate.

Linux job `rust-quality-and-smoke-benchmarks`: fmt, denied-warning Clippy, debug
and release workspace tests, MSRV1.95, doctests, denied-warning documentation,
locked Cargo metadata/tree for dependency review, existing primitive benchmark
smokes plus representative100-person smoke. No M5 latency or RSS threshold in CI.
The pre-existing REM-002 custom dependency-direction test removal was explicitly
approved; metadata/tree is evidence for manual architecture review, not an
invented equivalent automatic audit. No current test is removed to pass this task.

macOS job `godot-macos-integration`: official SHA512-verified Godot4.7.2,
native-RSS CLI measurement tests, GDExtension init, fresh editor import,
scene smoke, frame-statistics tests and CHRON031 snapshot/presentation integration.
Bash pipefail preserves engine exit failure, and script/error/crash output fails.
Headless checks explicitly cannot prove GPU readback, frame FPS, or rendered RSS;
those use local windowed evidence. Editor-exit crashes remain monitored and are
not hidden by a pipeline that returns tee's exit status.

The two protected job names remain unchanged. Candidate updates trigger one
pull_request run; push triggers only main, avoiding duplicated branch+PR suites.
No remote visibility or protection setting is modified.

## Evidence pending execution

Local commands and clean-checkout/hosted SHA/run links will be filled only after
execution. Source benchmark identities remain separate from CI candidate SHAs.
The reference ten-year run is reused across observational-only additions (ADR0029),
not repeated as CI. CHRON035 removal waits for both034 candidate checks to pass.

## Local evidence before candidate publication

Corpus creation ran the real runner twice per seed (generation invocation/series
files are retained). Reviewed schema2 hashes: seed0=17944541615393029991,
seed1=14837443214744277365, seed42=15313465221851226201. Every seed completed all
required behaviors for100persons; events1900/1900/1872 respectively.
`cargo test --release --locked -p palimpsest-headless-runner --test seed_regression`
passed the corpus; workspace all-target/all-feature denied-warning Clippy passed.
The two-day post-observer chaos report equals the repaired pre-observer report in
full, supporting reuse of the unchanged ten-year behavioral result (ADR0029).

Candidate publication includes pre-existing027–032/remediation implementation and
evidence needed to reproduce this workspace, not just the new benchmark files.
The pre-turn backup/source manifests retain attribution; no other branch is merged,
no main write/merge/force push or protection change is performed.
