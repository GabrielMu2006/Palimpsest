// Authored by opencode (AI coding agent) — P1-KERNEL-REPAIR KFIX-007.
//! Kernel throughput/advance benchmark (CHRON-028, ADR-0022; KFIX-007).
//!
//! Runs the deterministic headless kernel for a configured person population
//! and simulated horizon and reports, per warm-up/sample, the wall time,
//! sim-seconds-per-wall-second, advance rounds, transitions, decisions, and the
//! event total/digest. It emits one JSON object per invocation with the full
//! raw per-sample series and min/median/max, so the measurement protocol (two
//! warm-ups, ten samples by default) is reproducible.
//!
//! CLI:
//! ```sh
//! cargo run --release --locked -p palimpsest-sim-core --example kernel_bench \
//!     -- --persons 100 --seconds 86400 --warmups 2 --samples 10 --json
//! ```
//! `--json` forces the machine-readable output (diagnostics go to stderr).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use std::hint::black_box;
use std::time::Instant;

use palimpsest_sim_core::{KernelAdvance, KernelConfig, SimInstant, WorldKernel};
use palimpsest_sim_world::{WorldGenConfig, WorldMap, WorldSeed};
use serde::Serialize;

#[path = "support/bench_protocol.rs"]
mod protocol;

#[derive(Serialize)]
struct Sample {
    index: usize,
    wall_ns: u128,
    wall_seconds: f64,
    sim_per_wall: f64,
    rounds: usize,
    transitions: u64,
    decisions: u64,
    events_total: u64,
    events_digest: u64,
    checksum: u64,
    queue_max_observed: usize,
    queue_observation: &'static str,
    rounds_per_wall_second: f64,
    transitions_per_wall_second: f64,
    decisions_per_wall_second: f64,
    events_per_wall_second: f64,
}

#[derive(Serialize)]
struct Report {
    fixture: &'static str,
    seed: u64,
    persons: usize,
    seconds: i64,
    units: &'static str,
    warmups: usize,
    samples: usize,
    min_wall_seconds: f64,
    median_wall_ns: u128,
    max_wall_seconds: f64,
    median_sim_per_wall: f64,
    samples_series: Vec<Sample>,
    checksum: u64,
    spawn_layout: &'static str,
    queue_observation: &'static str,
    work_budget: usize,
    config: serde_json::Value,
}

/// Builds a seeded kernel with `persons` persons placed deterministically in a
/// walkable region; setup is excluded from the timed interval.
fn build(persons: usize, seed: u64) -> WorldKernel {
    let map = WorldMap::generate(WorldSeed::new(seed), WorldGenConfig::default());
    let origin = map
        .local()
        .coords()
        .find(|origin| {
            map.local()
                .get(origin.x(), origin.y())
                .is_some_and(|kind| kind.is_walkable())
        })
        .expect("generated map has a walkable spawn cell");
    let mut kernel = WorldKernel::from_world(WorldSeed::new(seed), KernelConfig::default());
    for _ in 0..persons {
        kernel
            .spawn_person(origin)
            .expect("identity capacity for benchmark population");
    }
    kernel.start_world(SimInstant::EPOCH).expect("start world");
    kernel
}

/// Drives the kernel to `seconds` and returns the last advance result and the
/// final truth checksum (a deterministic fold of the person views).
fn drive(kernel: &mut WorldKernel, seconds: i64) -> (KernelAdvance, u64, usize) {
    let target = SimInstant::from_seconds(seconds);
    let initial = kernel.metrics();
    let mut max_queue = kernel.metrics().scheduler_queue_depth;
    let mut result = kernel.advance(target).expect("bounded advance");
    max_queue = max_queue.max(kernel.metrics().scheduler_queue_depth);
    let mut totals = (
        result.rounds(),
        result.transitions(),
        result.decisions(),
        result.events(),
    );
    while !result.reached_target() {
        result = kernel.advance(target).expect("bounded advance");
        totals.0 += result.rounds();
        totals.1 += result.transitions();
        totals.2 += result.decisions();
        totals.3 += result.events();
        max_queue = max_queue.max(kernel.metrics().scheduler_queue_depth);
    }
    let metrics = kernel.metrics();
    assert_eq!(initial.rounds_total + totals.0 as u64, metrics.rounds_total);
    // Compare the full advance interval against cumulative kernel counters.
    // start_world's initial decisions are not part of these kernel counters.
    assert_eq!(
        initial.transitions_total + totals.1 as u64,
        metrics.transitions_total
    );
    assert_eq!(
        initial.decisions_total + totals.2 as u64,
        metrics.decisions_total
    );
    assert_eq!(initial.events_total + totals.3 as u64, metrics.events_total);
    let checksum = truth_checksum(kernel);
    (result, checksum, max_queue)
}

fn truth_checksum(kernel: &WorldKernel) -> u64 {
    let mut checksum = 0_u64;
    let views = kernel
        .persons()
        .expect("persons read at committed boundary");
    for byte in serde_json::to_vec(&views).expect("full person views serialize") {
        checksum = checksum
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::from(byte));
    }
    let sites = kernel.sites().expect("sites read at committed boundary");
    let site_bytes = serde_json::to_vec(sites).expect("sites serialize");
    for byte in site_bytes {
        checksum = checksum
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::from(byte));
    }
    let metrics = kernel.metrics();
    checksum = checksum
        .wrapping_mul(1_000_003)
        .wrapping_add(metrics.events_digest)
        .wrapping_add(metrics.events_total);
    checksum
}

fn run_sample(persons: usize, seed: u64, seconds: i64, index: usize) -> Sample {
    let mut kernel = build(persons, seed);
    let started = Instant::now();
    let (advance, checksum, queue_max_observed) = drive(&mut kernel, seconds);
    let elapsed = started.elapsed();
    let wall_ns = elapsed.as_nanos();
    let wall = elapsed.as_secs_f64();
    let metrics = kernel.metrics();
    // Correctness assertions stay enabled in every measured sample.
    assert!(
        advance.reached_target(),
        "the target must actually be reached"
    );
    let person_count = metrics.person_count;
    assert!(person_count == persons, "population preserved");
    Sample {
        index,
        wall_ns,
        wall_seconds: wall,
        sim_per_wall: seconds as f64 / wall,
        rounds: usize::try_from(metrics.rounds_total).expect("rounds fit usize"),
        transitions: metrics.transitions_total,
        decisions: metrics.decisions_total,
        events_total: metrics.events_total,
        events_digest: metrics.events_digest,
        checksum,
        queue_max_observed,
        queue_observation: "max scheduler depth at each completed advance boundary (not per-item peak)",
        rounds_per_wall_second: metrics.rounds_total as f64 / wall,
        transitions_per_wall_second: metrics.transitions_total as f64 / wall,
        decisions_per_wall_second: metrics.decisions_total as f64 / wall,
        events_per_wall_second: metrics.events_total as f64 / wall,
    }
}

fn main() {
    let mut defaults = protocol::defaults();
    defaults.persons = 100;
    let args = match protocol::parse(std::env::args().skip(1), defaults) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("kernel_bench: {e}");
            std::process::exit(2);
        }
    };
    if args.persons == 0 {
        eprintln!("kernel_bench: persons must be positive");
        std::process::exit(2);
    }
    let (persons, seconds, warmups, samples, seed, json) = (
        args.persons,
        args.seconds,
        args.warmups,
        args.samples,
        args.seed,
        args.json,
    );

    for index in 0..warmups {
        eprintln!("kernel_bench warmup {}/{}", index + 1, warmups);
        let warm = run_sample(persons, seed, seconds, index);
        black_box(warm);
    }

    let mut series = Vec::with_capacity(samples);
    for index in 0..samples {
        eprintln!("kernel_bench sample {}/{}", index + 1, samples);
        series.push(run_sample(persons, seed, seconds, index));
    }
    let mut walls: Vec<u128> = series.iter().map(|sample| sample.wall_ns).collect();
    let median_wall_ns = protocol::median(&mut walls);
    let median_wall = median_wall_ns as f64 / 1_000_000_000.0;
    let min_wall = series
        .iter()
        .map(|s| s.wall_seconds)
        .fold(f64::INFINITY, f64::min);
    let max_wall = series
        .iter()
        .map(|s| s.wall_seconds)
        .fold(0.0_f64, f64::max);
    let median_sim_per_wall = (seconds as f64) / median_wall;
    let checksum = series.last().expect("samples exist").checksum;
    let first = &series[0];
    for sample in &series[1..] {
        assert_eq!(
            (
                sample.transitions,
                sample.decisions,
                sample.events_total,
                sample.events_digest,
                sample.checksum
            ),
            (
                first.transitions,
                first.decisions,
                first.events_total,
                first.events_digest,
                first.checksum
            ),
            "truth differs between samples"
        );
        assert_eq!(
            (sample.rounds, sample.queue_max_observed),
            (first.rounds, first.queue_max_observed)
        );
    }

    let report = Report {
        fixture: "kernel-default-sites",
        seed,
        persons,
        seconds,
        units: "wall_ns=nanoseconds; rates=kernel advance counters per wall second; start_world excluded",
        warmups,
        samples,
        min_wall_seconds: min_wall,
        median_wall_ns,
        max_wall_seconds: max_wall,
        median_sim_per_wall,
        samples_series: series,
        checksum,
        spawn_layout: "colocated_first_walkable",
        queue_observation: "max scheduler depth at each completed advance boundary (not per-item peak)",
        work_budget: KernelConfig::default().work_budget(),
        config: protocol::configuration(),
    };
    let text = serde_json::to_string(&report).expect("serialize report");
    if json {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
}

/// Retains a kernel run for the memory benchmark adapter. The callback marks
/// the boundary; the kernel stays alive until the second observe. Returns a
/// full-truth checksum compared across cold samples.
///
/// Supported `case` values:
/// - `"100-year"` — 100 persons, seed 42, one simulated year (`31_536_000` s).
/// - `"10-year"` — 100 persons, seed 42, ten simulated years (`315_360_000` s);
///   the CHRON-032 peak-RSS measurement.
///
/// # Panics
///
/// Panics for any other `case`.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    let (persons, seconds) = match case {
        "100-year" | "10-year" => (
            100,
            if case == "10-year" {
                315_360_000
            } else {
                31_536_000
            },
        ),
        _ => panic!("invalid kernel memory workload selector: {case}"),
    };
    let mut kernel = build(persons, 42);
    observe();
    let (_, checksum, _) = drive(&mut kernel, seconds);
    observe();
    black_box(&kernel);
    checksum
}

#[cfg(test)]
mod tests {
    use super::{build, drive, memory_workload};
    use palimpsest_sim_core::SimInstant;

    #[test]
    fn colocated_fixture_reaches_target_and_accumulates_rounds() {
        let mut kernel = build(2, 42);
        let views = kernel.persons().expect("initial people");
        assert_eq!(views[0].location(), views[1].location());
        let (advance, checksum, queue) = drive(&mut kernel, 86_400);
        assert!(advance.reached_target());
        assert!(kernel.metrics().rounds_total >= advance.rounds() as u64);
        assert_eq!(kernel.metrics().person_count, 2);
        assert!(queue > 0);
        assert_ne!(checksum, 0);
        assert_eq!(kernel.metrics().now, SimInstant::from_seconds(86_400));
    }

    #[test]
    #[should_panic(expected = "invalid kernel memory workload selector")]
    fn memory_adapter_rejects_invalid_selector() {
        let _ = memory_workload("bad", &mut || {});
    }
}
