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

The GitHub Actions YAML parsed locally. Its pinned Godot download URL and
official SHA-512 manifest entry were verified reachable. It defines an Ubuntu
Rust quality/smoke job and an arm64 macOS Godot integration job.

## Evidence Boundary

The workspace had no Git repository or remote before this task. Local Git
metadata now exists, but no remote was invented and no commit or push was made.
Therefore the workflow is ready and locally equivalent gates pass, but there is
not yet a hosted GitHub Actions run to cite.
