//! Same-fixture unpaced direct/worker comparison; polling latency is explicit.
#![allow(clippy::cast_precision_loss)]
use palimpsest_headless_runner::micro_bench::{advance_to_target, build_fixture, validate_kernel};
use palimpsest_sim_core::{
    CommandOutcome, CommandStatus, RenderSnapshot, SimInstant, SimulationWorker, WorkerCommand,
};
use serde_json::json;
use std::time::{Duration, Instant};
fn sample() -> Result<serde_json::Value, String> {
    let mut direct = build_fixture(100, 86_400)?;
    let began = Instant::now();
    advance_to_target(&mut direct, 86_400)?;
    let direct_ns = began.elapsed().as_nanos();
    validate_kernel(&mut direct, 100, 86_400)?;
    let expected = RenderSnapshot::from_kernel(&direct).map_err(|e| e.to_string())?;
    let worker = SimulationWorker::new(build_fixture(100, 86_400)?).map_err(|e| e.to_string())?;
    let began = Instant::now();
    let sequence = worker
        .submit(WorkerCommand::AdvanceTo(SimInstant::from_seconds(86_400)))
        .map_err(|e| e.to_string())?;
    loop {
        if let CommandStatus::Completed(ack) = worker.command_status(sequence) {
            if ack.outcome() != &CommandOutcome::Applied
                || ack.committed_to().as_seconds() != 86_400
            {
                return Err(format!("unexpected ack: {ack:?}"));
            }
            break;
        }
        if began.elapsed() > Duration::from_secs(60) {
            return Err("worker acknowledgement timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let ack_ns = began.elapsed().as_nanos();
    let observed = worker.observe();
    if *observed.publication.snapshot != expected {
        return Err("direct and worker render DTO/work counters differ".into());
    }
    let published_ns = observed
        .publication
        .published_at
        .saturating_duration_since(began)
        .as_nanos();
    if published_ns > ack_ns {
        return Err("publication after observed acknowledgement".into());
    }
    Ok(
        json!({"direct_ns":direct_ns,"worker_ack_observed_ns":ack_ns,"worker_publication_ns":published_ns,
       "hash":expected.diagnostic_hash().to_string(),"metrics":expected.metrics(),"seconds":86400,"persons":100}),
    )
}
fn run() -> Result<(), String> {
    let mut warmups = Vec::new();
    let mut samples = Vec::new();
    for _ in 0..2 {
        warmups.push(sample()?);
    }
    for _ in 0..10 {
        samples.push(sample()?);
    }
    if warmups
        .iter()
        .chain(&samples)
        .any(|x| x["hash"] != samples[0]["hash"] || x["metrics"] != samples[0]["metrics"])
    {
        return Err("deterministic output mismatch".into());
    }
    let mut summary = serde_json::Map::new();
    for key in [
        "direct_ns",
        "worker_ack_observed_ns",
        "worker_publication_ns",
    ] {
        let mut times: Vec<f64> = samples
            .iter()
            .map(|x| x[key].as_u64().unwrap() as f64)
            .collect();
        times.sort_by(f64::total_cmp);
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        summary.insert(key.into(),json!({"median":times[times.len()/2],"min":times[0],"max":times[times.len()-1],"variance":times.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/times.len()as f64}));
    }
    println!(
        "{}",
        json!({"schema_version":1,"method":"matching seed42 reachable fixture; submit to ack observed with 1ms polling, publication own timestamp","warmups":warmups,"samples":samples,"summary":summary})
    );
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
