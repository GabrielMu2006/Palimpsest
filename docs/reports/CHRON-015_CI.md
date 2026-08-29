# CHRON-015 CI Validation

Date: 2026-08-29

## Gates

The local `tools/ci-rust.sh` entry point completed successfully with:

- read-only `MASTER_SPEC.md` SHA-256 verification;
- rustfmt check;
- Clippy `-D warnings` across the entire workspace and all targets/features;
- all workspace tests and targets;
- Rust 1.95.0 MSRV check;
- correctness-asserting smoke workloads for the headless runner, Scheduler,
  10K ECS entities, structured events, SQLite Event Store, and snapshots.
- the shared Headless/Rendered workload as a bounded smoke probe.

The local `tools/ci-godot.sh` entry point built the GDExtension, installed the
dynamic library into the Godot project, initialized godot-rust under Godot 4.7.2,
ran the complete scene for 30 headless frames, and observed no engine errors.
The deterministic `.godot/extension_list.cfg` is tracked so a clean checkout can
discover the native extension before parsing scripts that reference its class;
all other Godot editor/import cache remains ignored.

The GitHub Actions workflow runs an Ubuntu Rust quality/smoke job and an arm64
macOS Godot integration job. Its pinned Godot download and official SHA-512
manifest verification passed on the hosted macOS runner.

## Evidence Boundary

The private GitHub remote is
[`GabrielMu2006/Palimpsest`](https://github.com/GabrielMu2006/Palimpsest).
Hosted run
[`33241747464`](https://github.com/GabrielMu2006/Palimpsest/actions/runs/33241747464)
passed both jobs from commit `40bcd9c`: Rust/Linux in 6m17s and Godot/macOS in
34s. Two earlier runs exposed clean-checkout GDExtension discovery and MSRV
`rustfmt` provisioning omissions; both were fixed without removing, skipping,
or weakening tests.
