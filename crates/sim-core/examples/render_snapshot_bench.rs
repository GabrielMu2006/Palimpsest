// Authored by opencode (AI coding agent) — P1-KERNEL-REPAIR KFIX-007.
//! Render snapshot build + serialize benchmark (CHRON-029, ADR-0023; KFIX-007).
//!
//! Builds a schema-2 [`RenderSnapshot`] from a running kernel for a configured
//! person population and reports, per warm-up/sample, the build and serialize
//! wall times, the total serialized bytes, the per-section byte counts, and
//! the persons-array bytes/person (excluding terrain/sites). It emits one JSON
//! object with the full raw per-sample series plus min/median/max.
//!
//! CLI:
//! ```sh
//! cargo run --release --locked -p palimpsest-sim-core --example render_snapshot_bench \
//!     -- --persons 100 --warmups 2 --samples 10 --json
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
//! ```

use std::hint::black_box;
use std::time::Instant;

use palimpsest_sim_core::{KernelConfig, RenderSnapshot, SimInstant, WorldKernel};
use palimpsest_sim_world::{WorldGenConfig, WorldMap, WorldSeed};
use serde::Serialize;
#[path = "support/bench_protocol.rs"]
mod protocol;

const SEED: u64 = 42;

#[derive(Serialize)]
struct Sample {
    index: usize,
    build_ns: u128,
    serialize_ns: u128,
    build_us: f64,
    serialize_us: f64,
    total_bytes: usize,
    terrain_bytes: usize,
    sites_bytes: usize,
    persons_bytes: usize,
    metrics_bytes: usize,
    envelope_bytes: usize,
    per_person_bytes: Option<f64>,
    checksum: u64,
}

#[derive(Serialize)]
struct Report {
    fixture: &'static str,
    seed: u64,
    persons: usize,
    schema_version: u16,
    units: &'static str,
    warmups: usize,
    samples: usize,
    config: serde_json::Value,
    seconds: i64,
    spawn_layout: &'static str,
    min_build_us: f64,
    median_build_us: f64,
    max_build_us: f64,
    min_serialize_us: f64,
    median_serialize_us: f64,
    max_serialize_us: f64,
    median_total_bytes: usize,
    checksum: u64,
    samples_series: Vec<Sample>,
}

/// Builds a seeded, started kernel with `persons` persons and advances it to
/// 600 seconds so the snapshot reflects real committed action state.
fn build_kernel(persons: usize) -> WorldKernel {
    let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
    let walkable: Vec<palimpsest_sim_world::LocalCoord> = map
        .local()
        .coords()
        .filter(|coord| {
            map.local()
                .get(coord.x(), coord.y())
                .is_some_and(|k| k.is_walkable())
        })
        .collect();
    let mut kernel = WorldKernel::from_world(WorldSeed::new(SEED), KernelConfig::default());
    for index in 0..persons {
        let cell = walkable[index % walkable.len()];
        kernel
            .spawn_person(cell)
            .expect("identity capacity for benchmark population");
    }
    kernel.start_world(SimInstant::EPOCH).expect("start world");
    let target = SimInstant::from_seconds(600);
    let mut result = kernel.advance(target).expect("bounded advance");
    while !result.reached_target() {
        result = kernel.advance(target).expect("bounded advance");
    }
    kernel
}

fn section_bytes(value: &serde_json::Value, key: &str) -> usize {
    serde_json::to_vec(&value[key])
        .expect("serialize section")
        .len()
}
fn content_checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn run_sample(persons: usize, index: usize) -> Sample {
    let kernel = build_kernel(persons);
    let start = Instant::now();
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    let build_time = start.elapsed();
    let build_us = build_time.as_secs_f64() * 1_000_000.0;
    let start = Instant::now();
    let serialized = serde_json::to_vec(&snapshot).expect("serialize snapshot");
    let serialize_time = start.elapsed();
    let serialize_us = serialize_time.as_secs_f64() * 1_000_000.0;
    let total_bytes = serialized.len();
    let value: serde_json::Value = serde_json::from_slice(&serialized).expect("parse");
    let terrain_bytes = section_bytes(&value, "terrain");
    let sites_bytes = section_bytes(&value, "sites");
    let persons_bytes = section_bytes(&value, "persons");
    let metrics_bytes = section_bytes(&value, "metrics");
    let per_person_bytes = if persons == 0 {
        None
    } else {
        Some(persons_bytes as f64 / persons as f64)
    };
    // Correctness: the schema and person count match, every event is validated.
    assert_eq!(snapshot.schema_version(), 2);
    assert_eq!(snapshot.person_count(), persons);
    assert!(snapshot.validate().is_ok());
    verify_snapshot(&kernel, &snapshot, &serialized);
    let checksum = content_checksum(&serialized);
    Sample {
        index,
        build_ns: build_time.as_nanos(),
        serialize_ns: serialize_time.as_nanos(),
        build_us,
        serialize_us,
        total_bytes,
        terrain_bytes,
        sites_bytes,
        persons_bytes,
        metrics_bytes,
        envelope_bytes: total_bytes - terrain_bytes - sites_bytes - persons_bytes - metrics_bytes,
        per_person_bytes,
        checksum,
    }
}

fn main() {
    let mut defaults = protocol::defaults();
    defaults.seconds = 600;
    let args = match protocol::parse_for(
        std::env::args().skip(1),
        defaults,
        &["--persons", "--samples", "--warmups", "--json"],
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("render_snapshot_bench: {e}");
            std::process::exit(2);
        }
    };
    let (persons, warmups, samples, json) = (args.persons, args.warmups, args.samples, args.json);

    for index in 0..warmups {
        eprintln!("render warmup {}/{}", index + 1, warmups);
        black_box(run_sample(persons, index));
    }

    let mut series = Vec::with_capacity(samples);
    for index in 0..samples {
        eprintln!("render sample {}/{}", index + 1, samples);
        series.push(run_sample(persons, index));
    }
    let first = &series[0];
    for sample in &series[1..] {
        assert_eq!(
            (
                sample.checksum,
                sample.total_bytes,
                sample.persons_bytes,
                sample.sites_bytes,
                sample.metrics_bytes
            ),
            (
                first.checksum,
                first.total_bytes,
                first.persons_bytes,
                first.sites_bytes,
                first.metrics_bytes
            )
        );
    }
    let mut builds: Vec<u128> = series.iter().map(|s| s.build_ns).collect();
    let mut serializes: Vec<u128> = series.iter().map(|s| s.serialize_ns).collect();
    let median_build_us = protocol::median(&mut builds) as f64 / 1_000.0;
    let median_serialize_us = protocol::median(&mut serializes) as f64 / 1_000.0;
    let mut totals: Vec<usize> = series.iter().map(|s| s.total_bytes).collect();
    totals.sort_unstable();
    let median_total_bytes = totals[totals.len() / 2];
    let checksum = series.last().expect("samples exist").checksum;

    let report = Report {
        fixture: "render-seed42-kernel-at-600s",
        seed: SEED,
        persons,
        schema_version: 2,
        units: "microseconds / bytes",
        warmups,
        samples,
        config: protocol::configuration(),
        seconds: 600,
        spawn_layout: "first_row_major_walkable_cells",
        min_build_us: builds[0] as f64 / 1_000.0,
        median_build_us,
        max_build_us: builds[builds.len() - 1] as f64 / 1_000.0,
        min_serialize_us: serializes[0] as f64 / 1_000.0,
        median_serialize_us,
        max_serialize_us: serializes[serializes.len() - 1] as f64 / 1_000.0,
        median_total_bytes,
        checksum,
        samples_series: series,
    };
    let text = serde_json::to_string(&report).expect("serialize report");
    if json {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
}

/// A prepared 600-second kernel shared by the two render memory selectors so
/// the control and snapshot runs measure the same fixture.
fn prepared(persons: usize) -> WorldKernel {
    build_kernel(persons)
}

fn readonly_checksum(kernel: &WorldKernel) -> u64 {
    let views = kernel.persons().expect("running read");
    assert_eq!(kernel.now(), SimInstant::from_seconds(600));
    assert_eq!(views.len(), kernel.person_count());
    content_checksum(&serde_json::to_vec(&views).expect("serialize read-only truth"))
}

fn verify_snapshot(kernel: &WorldKernel, snapshot: &RenderSnapshot, bytes: &[u8]) {
    let decoded: RenderSnapshot = serde_json::from_slice(bytes).expect("validated roundtrip");
    assert_eq!(&decoded, snapshot);
    assert_eq!(snapshot.sim_second(), kernel.now());
    let views = kernel.persons().expect("committed people");
    assert_eq!(snapshot.person_count(), views.len());
    for (presented, truth) in snapshot.persons().iter().zip(views) {
        assert_eq!(presented.person_id(), truth.id());
        assert_eq!(presented.tile(), truth.location());
        assert_eq!(presented.needs(), truth.needs());
        assert_eq!(presented.action(), truth.action());
        assert_eq!(presented.action_target(), truth.action_target());
        assert_eq!(presented.action_state(), truth.state());
    }
    assert_eq!(snapshot.sites().len(), kernel.sites().unwrap().len());
    for site in snapshot.sites() {
        assert_eq!(
            kernel
                .sites()
                .unwrap()
                .site_at(site.coord())
                .unwrap()
                .kind(),
            site.kind()
        );
    }
    assert_eq!(
        snapshot.terrain().cells(),
        &kernel.map().local().iter().copied().collect::<Vec<_>>()
    );
    assert_eq!(
        snapshot.metrics().events_committed,
        kernel.metrics().events_total
    );
}

/// Retains a prepared kernel (no snapshot) for the control memory selector.
///
/// # Panics
///
/// Panics when `case` is not `"control-100"` or `"snapshot-100"`.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    match case {
        "control-100" => {
            let kernel = prepared(100);
            observe();
            let checksum = readonly_checksum(&kernel);
            observe();
            black_box(&kernel);
            checksum
        }
        "snapshot-100" => {
            let kernel = prepared(100);
            observe();
            let checksum = readonly_checksum(&kernel);
            let snapshot = RenderSnapshot::from_kernel(&kernel).expect("snapshot");
            let bytes = serde_json::to_vec(&snapshot).expect("serialize");
            assert_eq!(snapshot.person_count(), 100);
            verify_snapshot(&kernel, &snapshot, &bytes);
            black_box((&kernel, &snapshot, &bytes));
            observe();
            black_box((&kernel, &snapshot, &bytes));
            checksum
        }
        other => panic!("invalid render memory workload selector: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{memory_workload, run_sample};

    #[test]
    fn zero_people_have_null_ratio_and_exact_section_accounting() {
        let sample = run_sample(0, 0);
        assert_eq!(sample.per_person_bytes, None);
        assert_eq!(sample.persons_bytes, 2);
        assert_eq!(
            sample.total_bytes,
            sample.terrain_bytes
                + sample.sites_bytes
                + sample.persons_bytes
                + sample.metrics_bytes
                + sample.envelope_bytes
        );
        let json = serde_json::to_value(sample).unwrap();
        assert!(json["per_person_bytes"].is_null());
    }

    #[test]
    fn snapshot_and_readonly_control_have_identical_underlying_truth() {
        assert_eq!(
            memory_workload("control-100", &mut || {}),
            memory_workload("snapshot-100", &mut || {})
        );
    }

    #[test]
    fn memory_adapter_control_observes_twice() {
        let mut calls = 0;
        let _ = memory_workload("control-100", &mut || calls += 1);
        assert_eq!(calls, 2);
    }

    #[test]
    fn memory_adapter_snapshot_observes_twice() {
        let mut calls = 0;
        let _ = memory_workload("snapshot-100", &mut || calls += 1);
        assert_eq!(calls, 2);
    }

    #[test]
    #[should_panic(expected = "invalid render memory workload selector")]
    fn memory_adapter_rejects_invalid_selector() {
        let _ = memory_workload("bad", &mut || {});
    }
}
