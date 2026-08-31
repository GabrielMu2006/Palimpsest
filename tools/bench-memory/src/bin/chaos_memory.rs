//! One cold, native-RSS observation of the exact CHRON-032 chaos workload.
//! This outward adapter reuses ADR-0020; no native code is added to Core.
#![allow(clippy::cast_precision_loss)]

#[allow(dead_code)]
#[path = "../rss.rs"]
mod rss;

use palimpsest_sim_core::{ChaosCheckpoint, ChaosConfig, run_chaos_observed};
use serde_json::json;
use std::process::ExitCode;
use std::time::Instant;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seconds = match args.as_slice() {
        [] => 315_360_000,
        [flag, value] if flag == "--seconds" => value.parse::<i64>().map_err(|e| e.to_string())?,
        _ => return Err("usage: chaos_memory [--seconds positive-i64]".into()),
    };
    if seconds <= 0 {
        return Err("seconds must be positive".into());
    }
    let config = ChaosConfig {
        seed: 42,
        person_count: 100,
        years: if seconds % 31_536_000 == 0 {
            (seconds / 31_536_000).unsigned_abs()
        } else {
            1
        },
        sim_seconds_per_year: if seconds % 31_536_000 == 0 {
            31_536_000
        } else {
            seconds
        },
    };
    // Prime only the measurement syscall, not the fixture or simulation.
    rss::read()?;
    let baseline = rss::read()?;
    let mut prepared = None;
    let mut end = None;
    let mut failure = None;
    let mut trend = Vec::new();
    let started = Instant::now();
    let report = run_chaos_observed(&config, true, &mut |checkpoint, kernel| {
        if checkpoint == ChaosCheckpoint::Advance { return; }
        match rss::read() {
            Ok(reading) => match checkpoint {
                ChaosCheckpoint::Prepared => prepared = Some(reading),
                ChaosCheckpoint::Complete => end = Some(reading),
                ChaosCheckpoint::Day => {
                    trend.push(json!({"seconds": kernel.now().as_seconds(), "current_rss_bytes":reading.current_bytes}));
                    if trend.len() % 365 == 0 { eprintln!("chaos RSS: {} days, {:.1}s wall", trend.len(), started.elapsed().as_secs_f64()); }
                }
                ChaosCheckpoint::Advance => {}
            },
            Err(error) => failure = Some(error),
        }
    }).map_err(|e| e.to_string())?;
    if let Some(error) = failure {
        return Err(error);
    }
    let end = end.ok_or("missing completed observation")?;
    let cold = rss::Interval::between(baseline, end)?;
    let operation = rss::Interval::between(prepared.ok_or("missing prepared observation")?, end)?;
    if cold.peak_increment_bytes.is_none() {
        return Err("unprovable cold peak; no silent retry".into());
    }
    let output = json!({"schema_version":1, "method":"macos_kernel_rss_high_water_v1", "samples":1,
        "workload":"run_chaos_observed", "wall_seconds":started.elapsed().as_secs_f64(),
        "cold":cold,"operation":operation,"daily_rss_trend":trend,
        "trend_note":"current RSS at daily checkpoints; not an instantaneous peak", "report":report});
    println!(
        "{}",
        serde_json::to_string(&output).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chaos_memory: {error}");
            ExitCode::FAILURE
        }
    }
}
