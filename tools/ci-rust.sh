#!/usr/bin/env bash
set -euo pipefail

expected_spec_hash="a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea"
if command -v sha256sum >/dev/null 2>&1; then
    actual_spec_hash=$(sha256sum MASTER_SPEC.md | awk '{print $1}')
else
    actual_spec_hash=$(shasum -a 256 MASTER_SPEC.md | awk '{print $1}')
fi
if [ "$actual_spec_hash" != "$expected_spec_hash" ]; then
    echo "MASTER_SPEC.md differs from the Phase 0 read-only baseline" >&2
    exit 1
fi

cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo +1.95.0 check --locked --workspace --all-targets --all-features

cargo test --release --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
# Reproducible dependency graph for review; not the retired custom dependency test.
cargo metadata --locked --format-version 1 > "${TMPDIR:-/tmp}/palimpsest-cargo-metadata.json"
cargo tree --locked --workspace --edges normal
# This is correctness smoke only. Reference-machine throughput/RSS remains local.
cargo run --release --locked -p palimpsest-headless-runner --bin bench_micro_world -- \
    --scales 100 --seconds 86400 --warmups 0 --samples 1 \
    | python3 -c 'import json,sys; r=json.load(sys.stdin); assert r["status"]=="passed" and len(r["samples"])==1'

cargo run --release --locked -p palimpsest-headless-runner --bin palimpsest-headless-runner -- \
    --entities 100 --seconds 86400 \
    | python3 -c 'import json,sys; r=json.load(sys.stdin); assert r["entities"]==100 and r["final_sim_second"]==86400 and r["generated_events"]>0 and r["remaining_scheduled"]>0'
cargo run --release --locked -p palimpsest-sim-scheduler --example scheduler_bench -- 10000 2 \
    | python3 -c 'import json,sys; assert json.load(sys.stdin)["items"] == 10000'
cargo run --release --locked -p palimpsest-headless-runner --bin bench_10k_entities -- 10000 10 \
    | python3 -c 'import json,sys; assert json.load(sys.stdin)["stable_mapping_entries"] == 10000'
cargo run --release --locked -p palimpsest-headless-runner --bin bench_event_throughput -- 10000 2 \
    | python3 -c 'import json,sys; assert json.load(sys.stdin)["events"] == 10000'
cargo run --release --locked -p palimpsest-sim-storage --example event_store_bench -- 10000 1000 \
    | python3 -c 'import json,sys; assert json.load(sys.stdin)["events"] == 10000'
cargo run --release --locked -p palimpsest-sim-storage --example snapshot_bench -- 1000 2 \
    | python3 -c 'import json,sys; assert json.load(sys.stdin)["entities"] == 1000'
