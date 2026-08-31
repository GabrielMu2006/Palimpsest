// Authored by Kimi Code (AI coding agent) — task CHRON-030 (ADR-0015 supplement).
//! Simulation-worker throughput/overhead benchmark (CHRON-030).
//!
//! Compares a direct-kernel control with the CHRON-030 worker over the same
//! deterministic 100-person fixture, seed, and end instant: per sample it
//! measures the direct advance wall time, the worker `AdvanceTo`
//! submit-to-ack wall time, the submit-to-publication latency, and a paused
//! command-throughput flood. Correctness assertions stay enabled in every
//! sample, and the worker's final published snapshot must equal the direct
//! control's snapshot checksum. A separate short real-wall-clock pacing pass
//! reports the observed sim-seconds per wall-second for each multiplier; it
//! is a pacing diagnostic, not a throughput claim (P1-REMAINING §4).
//!
//! CLI:
//! ```sh
//! cargo run --release --locked -p palimpsest-sim-core --example worker_bench \
//!     -- --persons 100 --seconds 86400 --warmups 2 --samples 10 --json
//! ```
//! `--json` forces the machine-readable output (diagnostics go to stderr).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use palimpsest_sim_core::{
    CommandStatus, KernelConfig, RenderSnapshot, SimInstant, SimulationWorker, SpeedMultiplier,
    WorkerCommand, WorldKernel,
};
use palimpsest_sim_world::{WorldGenConfig, WorldMap, WorldSeed};
use serde::Serialize;

#[path = "support/bench_protocol.rs"]
mod protocol;

/// Commands issued per command-throughput flood sample.
const THROUGHPUT_COMMANDS: usize = 512;

/// Builds the same colocated first-walkable fixture as `kernel_bench`
/// (CHRON-028); setup is excluded from the timed intervals.
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

/// Deterministic checksum of a published snapshot (serde JSON byte fold).
fn snapshot_checksum(snapshot: &RenderSnapshot) -> u64 {
    snapshot.diagnostic_hash()
}

/// Direct-kernel control: advance to the target and return the wall time and
/// the final snapshot checksum.
fn direct_control(persons: usize, seed: u64, seconds: i64) -> (Duration, u64) {
    let mut kernel = build(persons, seed);
    let target = SimInstant::from_seconds(seconds);
    let started = Instant::now();
    let mut result = kernel.advance(target).expect("bounded advance");
    while !result.reached_target() {
        result = kernel.advance(target).expect("bounded advance");
    }
    let wall = started.elapsed();
    assert_eq!(kernel.now(), target, "direct control reached the target");
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("snapshot of a complete boundary");
    (wall, snapshot_checksum(&snapshot))
}

/// Waits for one command's final acknowledgement.
fn wait_ack(worker: &SimulationWorker, sequence: palimpsest_sim_core::CommandSequence) {
    loop {
        if let CommandStatus::Completed(_) = worker.command_status(sequence) {
            return;
        }
        std::thread::yield_now();
    }
}

/// Worker run: submit one `AdvanceTo` to the target from the paused initial
/// state and measure submit-to-ack wall, submit-to-visible-publication wall,
/// and the final snapshot checksum.
fn worker_run(persons: usize, seed: u64, seconds: i64) -> (Duration, Duration, u64, usize) {
    let worker = SimulationWorker::new(build(persons, seed)).expect("worker starts");
    let target = SimInstant::from_seconds(seconds);
    let started = Instant::now();
    let sequence = worker
        .submit(WorkerCommand::AdvanceTo(target))
        .expect("enqueue advance");
    // An ack observed after a publication poll can race that earlier poll.
    // Read again after the ack, and use the publication's own timestamp.
    let ack_wall = loop {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "worker acknowledgement timeout"
        );
        if let CommandStatus::Completed(ack) = worker.command_status(sequence) {
            assert_eq!(ack.outcome(), &palimpsest_sim_core::CommandOutcome::Applied);
            assert_eq!(ack.committed_to(), target);
            break started.elapsed();
        }
        std::thread::yield_now();
    };
    let observed = worker.observe();
    assert_eq!(observed.publication.snapshot.sim_second(), target);
    let publication_wall = observed
        .publication
        .published_at
        .saturating_duration_since(started);
    assert!(
        publication_wall <= ack_wall,
        "publication precedes acknowledged completion"
    );
    let checksum = snapshot_checksum(&worker.latest_snapshot());
    let max_queue = worker.status().max_queue_depth;
    (ack_wall, publication_wall, checksum, max_queue)
}

/// Paused command throughput: keep the bounded queue saturated with no-op
/// steps until `count` commands are acknowledged; returns commands/wall-s and
/// the maximum observed queue depth.
fn command_throughput(persons: usize, seed: u64, count: usize) -> (f64, usize) {
    let worker = SimulationWorker::new(build(persons, seed)).expect("worker starts");
    let capacity = palimpsest_sim_core::COMMAND_QUEUE_CAPACITY;
    let started = Instant::now();
    let mut submitted = 0_usize;
    let mut acknowledged = 0_usize;
    let mut pending = Vec::new();
    while acknowledged < count {
        while submitted < count && pending.len() < capacity {
            let sequence = worker
                .submit(WorkerCommand::Step(0))
                .expect("queue has room by construction");
            pending.push(sequence);
            submitted += 1;
        }
        let oldest = pending.first().copied().expect("a command is in flight");
        wait_ack(&worker, oldest);
        pending.remove(0);
        acknowledged += 1;
    }
    let wall = started.elapsed().as_secs_f64();
    let max_queue = worker.status().max_queue_depth;
    let status = worker.status();
    assert_eq!(
        status.commands_applied, count as u64,
        "every no-op step was applied"
    );
    (count as f64 / wall, max_queue)
}

#[derive(Serialize)]
struct PacingSample {
    speed: &'static str,
    window_wall_ms: u64,
    sim_seconds_advanced: i64,
    observed_sim_per_wall: f64,
    nominal_sim_per_wall: Option<u64>,
}

/// Short real-wall-clock pacing diagnostic for each multiplier. This uses
/// short controlled windows rather than waiting a day at 1x (P1-REMAINING §4)
/// and is not a reproducible input trace.
fn pacing_diagnostics(persons: usize, seed: u64) -> Vec<PacingSample> {
    let speeds = [
        ("1", SpeedMultiplier::X1, 300_u64),
        ("5", SpeedMultiplier::X5, 300),
        ("20", SpeedMultiplier::X20, 300),
        ("100", SpeedMultiplier::X100, 300),
        ("1000", SpeedMultiplier::X1000, 300),
        ("MAX", SpeedMultiplier::Max, 100),
    ];
    let mut out = Vec::new();
    for (name, speed, window_ms) in speeds {
        let worker = SimulationWorker::new(build(persons, seed)).expect("worker starts");
        worker
            .submit(WorkerCommand::SetSpeed(speed))
            .expect("enqueue speed");
        let resume = worker
            .submit(WorkerCommand::Resume)
            .expect("enqueue resume");
        wait_ack(&worker, resume);
        let anchor = worker.status().committed;
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(window_ms));
        let pause = worker.submit(WorkerCommand::Pause).expect("enqueue pause");
        wait_ack(&worker, pause);
        let wall = started.elapsed().as_secs_f64();
        let advanced = worker.status().committed.as_seconds() - anchor.as_seconds();
        out.push(PacingSample {
            speed: name,
            window_wall_ms: window_ms,
            sim_seconds_advanced: advanced,
            observed_sim_per_wall: advanced as f64 / wall,
            nominal_sim_per_wall: speed.factor(),
        });
    }
    out
}

#[derive(Serialize)]
struct Sample {
    index: usize,
    direct_wall_ns: u128,
    worker_wall_ns: u128,
    publication_latency_ns: u128,
    worker_overhead_ratio: f64,
    sim_per_wall_direct: f64,
    sim_per_wall_worker: f64,
    commands_per_second: f64,
    max_queue_depth: usize,
    checksum: u64,
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
    direct_wall_ns_min: u128,
    direct_wall_ns_median: u128,
    direct_wall_ns_max: u128,
    worker_wall_ns_min: u128,
    worker_wall_ns_median: u128,
    worker_wall_ns_max: u128,
    publication_latency_ns_median: u128,
    overhead_ratio_median: f64,
    commands_per_second_median: f64,
    max_queue_depth_observed: usize,
    checksum: u64,
    samples_series: Vec<Sample>,
    pacing: Vec<PacingSample>,
    config: serde_json::Value,
}

fn run_sample(persons: usize, seed: u64, seconds: i64, index: usize) -> Sample {
    let (direct_wall, direct_checksum) = direct_control(persons, seed, seconds);
    let (ack_wall, publication_wall, worker_checksum, _) = worker_run(persons, seed, seconds);
    assert_eq!(
        direct_checksum, worker_checksum,
        "worker and direct control commit identical truth"
    );
    let (commands_per_second, max_queue_depth) =
        command_throughput(persons, seed, THROUGHPUT_COMMANDS);
    Sample {
        index,
        direct_wall_ns: direct_wall.as_nanos(),
        worker_wall_ns: ack_wall.as_nanos(),
        publication_latency_ns: publication_wall.as_nanos(),
        worker_overhead_ratio: ack_wall.as_secs_f64() / direct_wall.as_secs_f64(),
        sim_per_wall_direct: seconds as f64 / direct_wall.as_secs_f64(),
        sim_per_wall_worker: seconds as f64 / ack_wall.as_secs_f64(),
        commands_per_second,
        max_queue_depth,
        checksum: worker_checksum,
    }
}

fn main() {
    let mut defaults = protocol::defaults();
    defaults.persons = 100;
    let args = match protocol::parse(std::env::args().skip(1), defaults) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("worker_bench: {e}");
            std::process::exit(2);
        }
    };
    if args.persons == 0 {
        eprintln!("worker_bench: persons must be positive");
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
        eprintln!("worker_bench warmup {}/{}", index + 1, warmups);
        let warm = run_sample(persons, seed, seconds, index);
        black_box(warm);
    }

    let mut series = Vec::with_capacity(samples);
    for index in 0..samples {
        eprintln!("worker_bench sample {}/{}", index + 1, samples);
        series.push(run_sample(persons, seed, seconds, index));
    }
    let first = &series[0];
    for sample in &series[1..] {
        assert_eq!(
            sample.checksum, first.checksum,
            "truth differs between samples"
        );
    }
    let mut direct: Vec<u128> = series.iter().map(|s| s.direct_wall_ns).collect();
    let mut worker: Vec<u128> = series.iter().map(|s| s.worker_wall_ns).collect();
    let mut latency: Vec<u128> = series.iter().map(|s| s.publication_latency_ns).collect();
    let mut commands: Vec<u64> = series
        .iter()
        .map(|s| s.commands_per_second as u64)
        .collect();
    let overhead_median = {
        let mut ratios: Vec<u64> = series
            .iter()
            .map(|s| (s.worker_overhead_ratio * 1_000_000.0) as u64)
            .collect();
        protocol::median(&mut ratios) as f64 / 1_000_000.0
    };
    let pacing = pacing_diagnostics(persons, seed);

    let report = Report {
        fixture: "worker-vs-direct-colocated-first-walkable",
        seed,
        persons,
        seconds,
        units: "wall_ns=nanoseconds; overhead=worker_submit_to_ack/direct_advance; publication_latency=submit_to_visible_snapshot; commands/s=paused noop-step throughput; pacing=short wall windows (diagnostic)",
        warmups,
        samples,
        direct_wall_ns_min: *direct.iter().min().expect("samples exist"),
        direct_wall_ns_median: protocol::median(&mut direct),
        direct_wall_ns_max: *direct.iter().max().expect("samples exist"),
        worker_wall_ns_min: *worker.iter().min().expect("samples exist"),
        worker_wall_ns_median: protocol::median(&mut worker),
        worker_wall_ns_max: *worker.iter().max().expect("samples exist"),
        publication_latency_ns_median: protocol::median(&mut latency),
        overhead_ratio_median: overhead_median,
        commands_per_second_median: protocol::median(&mut commands) as f64,
        max_queue_depth_observed: series
            .iter()
            .map(|s| s.max_queue_depth)
            .max()
            .expect("samples exist"),
        checksum: first.checksum,
        samples_series: series,
        pacing,
        config: protocol::configuration(),
    };
    let text = serde_json::to_string(&report).expect("serialize report");
    if json {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
}

/// Retains a running 100-person worker advancing one day for the memory
/// benchmark adapter. The callback marks the prepared/operation boundaries;
/// the worker (thread, channels, last snapshot) stays alive until the second
/// observe. Returns the published snapshot checksum, stable across cold runs.
///
/// # Panics
///
/// Panics when `case` is not `"100-day"`.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    assert_eq!(
        case, "100-day",
        "invalid worker memory workload selector: {case}"
    );
    let worker = SimulationWorker::new(build(100, 42)).expect("worker starts");
    observe();
    let target = SimInstant::from_seconds(86_400);
    let sequence = worker
        .submit(WorkerCommand::AdvanceTo(target))
        .expect("enqueue advance");
    wait_ack(&worker, sequence);
    assert_eq!(
        worker.latest_snapshot().sim_second(),
        target,
        "published snapshot reached the target"
    );
    let checksum = snapshot_checksum(&worker.latest_snapshot());
    observe();
    black_box(&worker);
    checksum
}

#[cfg(test)]
mod tests {
    use super::{command_throughput, direct_control, memory_workload, worker_run};

    #[test]
    fn worker_matches_direct_control_on_a_short_horizon() {
        let (direct_wall, direct_checksum) = direct_control(2, 42, 600);
        let (ack_wall, publication_wall, worker_checksum, _) = worker_run(2, 42, 600);
        assert!(direct_wall.as_nanos() > 0);
        assert!(ack_wall >= publication_wall);
        assert_eq!(direct_checksum, worker_checksum);
    }

    #[test]
    fn throughput_flood_applies_every_command() {
        let (rate, max_queue) = command_throughput(2, 42, 64);
        assert!(rate > 0.0);
        assert!(max_queue <= palimpsest_sim_core::COMMAND_QUEUE_CAPACITY);
    }

    #[test]
    #[should_panic(expected = "invalid worker memory workload selector")]
    fn memory_adapter_rejects_invalid_selector() {
        let _ = memory_workload("bad", &mut || {});
    }
}
