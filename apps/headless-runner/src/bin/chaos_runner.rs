// Authored by opencode (AI coding agent) — task CHRON-032.
//! Headless 10-year chaos runner binary (CHRON-032, ADR-0027).
//!
//! Runs the deterministic `run_chaos` pipeline `--runs` times, times each run,
//! asserts cross-run determinism, enforces the Phase 1 completion gate (every
//! person completes real movement, Eat, Sleep and Work at least once), and writes a JSON + Markdown report.
//!
//! CLI:
//! ```sh
//! cargo run --release --locked -p palimpsest-headless-runner --bin chaos_runner -- \
//!     --seed 42 --persons 100 --years 10 --runs 3 --out docs/reports/data/chron-032-chaos.json
//! ```
//! Peak process RSS is measured separately by the native memory tool
//! (`chaos_memory`) using the same chaos fixture;
//! this binary records wall-clock timing only.
#![allow(clippy::cast_precision_loss)]

use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use palimpsest_sim_core::{
    ChaosCheckpoint, ChaosConfig, ChaosError, ChaosMeasurement, ChaosReport, SECONDS_PER_YEAR,
    run_chaos_observed,
};
use serde::Serialize;

#[derive(Serialize)]
struct RunRow {
    index: usize,
    wall_seconds: f64,
    sim_per_wall: f64,
    events_per_wall: f64,
    truth_hash: u64,
    per_person_digest: u64,
    events_total: u64,
    final_seconds: i64,
}

#[derive(Serialize)]
struct Output {
    config: ChaosConfig,
    runs: usize,
    min_wall_seconds: f64,
    median_wall_seconds: f64,
    max_wall_seconds: f64,
    runs_series: Vec<RunRow>,
    determinism: bool,
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} [--seed u64] [--persons usize] [--years u64] [--runs usize] [--seconds i64] [--out path] [--markdown path] [--watchdog-seconds u64] [--no-gate]"
    )
}

#[derive(Default)]
struct Args {
    seed: u64,
    persons: usize,
    years: u64,
    runs: usize,
    out: String,
    seconds_override: Option<i64>,
    gate: bool,
    markdown: Option<String>,
    watchdog_seconds: u64,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let program = args.next().unwrap_or_default();
    let mut out = Args {
        seed: 42,
        persons: 100,
        years: 10,
        runs: 3,
        out: "docs/reports/data/chron-032-chaos.json".to_string(),
        seconds_override: None,
        gate: true,
        markdown: None,
        watchdog_seconds: 60,
    };
    let mut iter = args;
    while let Some(flag) = iter.next() {
        let mut take = |one: &str| {
            iter.next()
                .ok_or_else(|| format!("{program}: {one} requires a value"))
        };
        match flag.as_str() {
            "--seed" => {
                out.seed = take("--seed")?
                    .parse()
                    .map_err(|_| format!("{program}: invalid --seed"))?;
            }
            "--persons" => {
                out.persons = take("--persons")?
                    .parse()
                    .map_err(|_| format!("{program}: invalid --persons"))?;
            }
            "--years" => {
                out.years = take("--years")?
                    .parse()
                    .map_err(|_| format!("{program}: invalid --years"))?;
            }
            "--runs" => {
                out.runs = take("--runs")?
                    .parse()
                    .map_err(|_| format!("{program}: invalid --runs"))?;
            }
            "--out" => out.out = take("--out")?,
            "--seconds" => {
                out.seconds_override = Some(
                    take("--seconds")?
                        .parse()
                        .map_err(|_| format!("{program}: invalid --seconds"))?,
                );
            }
            "--no-gate" => out.gate = false,
            "--help" | "-h" => return Err(format!("HELP\n{}", usage(&program))),
            "--markdown" => out.markdown = Some(take("--markdown")?),
            "--watchdog-seconds" => {
                out.watchdog_seconds = take("--watchdog-seconds")?
                    .parse()
                    .map_err(|_| format!("{program}: invalid --watchdog-seconds"))?;
            }
            other => {
                return Err(format!(
                    "{program}: unknown flag {other}\n{}",
                    usage(&program)
                ));
            }
        }
    }
    if out.persons == 0 {
        return Err(format!("{program}: --persons must be positive"));
    }
    if out.years == 0 {
        return Err(format!("{program}: --years must be positive"));
    }
    if out.runs == 0 {
        return Err(format!("{program}: --runs must be positive"));
    }
    if out.seconds_override.is_some_and(|seconds| seconds <= 0) {
        return Err(format!("{program}: --seconds must be positive"));
    }
    if out.watchdog_seconds == 0 {
        return Err(format!("{program}: --watchdog-seconds must be positive"));
    }
    Ok(out)
}

// A bounded heartbeat slot cannot accumulate a backlog that masks a stalled worker.
fn supervise<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce(&mut dyn FnMut()) -> Result<T, ChaosError> + Send + 'static,
) -> Result<T, ChaosError> {
    enum Message<T> {
        Progress,
        Result(Result<T, ChaosError>),
    }
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            work(&mut || {
                let _ = tx.try_send(Message::Progress);
            })
        }))
        .unwrap_or_else(|_| {
            Err(ChaosError::Invariant {
                rule: "worker panic",
                detail: "chaos worker panicked".into(),
            })
        });
        let _ = tx.send(Message::Result(result));
    });
    loop {
        match rx.recv_timeout(timeout) {
            Ok(Message::Result(result)) => return result,
            Ok(Message::Progress) => {}
            Err(error) => {
                return Err(ChaosError::Invariant {
                    rule: "watchdog",
                    detail: format!("worker did not return progress/result: {error}"),
                });
            }
        }
    }
}

fn supervised(config: ChaosConfig, timeout_seconds: u64) -> Result<ChaosReport, ChaosError> {
    supervise(Duration::from_secs(timeout_seconds), move |heartbeat| {
        run_chaos_observed(&config, false, &mut |_: ChaosCheckpoint, _| heartbeat())
    })
}

fn config_for(args: &Args) -> Result<ChaosConfig, ChaosError> {
    let mut config = ChaosConfig {
        seed: args.seed,
        person_count: args.persons,
        years: args.years,
        sim_seconds_per_year: SECONDS_PER_YEAR,
    };
    if let Some(seconds) = args.seconds_override {
        config.years = 1;
        config.sim_seconds_per_year = seconds;
    }
    config.target_seconds()?;
    Ok(config)
}

#[allow(clippy::too_many_lines)]
fn run(args: &Args) -> Result<(Output, ChaosReport), ChaosError> {
    let config = config_for(args)?;
    let target = config.target_seconds()?;
    let mut series = Vec::with_capacity(args.runs);
    let mut first_report: Option<ChaosReport> = None;

    for index in 0..args.runs {
        eprintln!("chaos_runner run {}/{}", index + 1, args.runs);
        let started = Instant::now();
        let report = supervised(config, args.watchdog_seconds)?;
        let wall = started.elapsed().as_secs_f64();
        let sim_per_wall = target as f64 / wall;
        let events_per_wall = report.events_total as f64 / wall;

        if let Some(first) = &first_report {
            if !first.deterministic_eq(&report) {
                return Err(ChaosError::Invariant {
                    rule: "cross-run determinism",
                    detail: format!("run {index} deterministic report differs from first"),
                });
            }
        } else {
            first_report = Some(report.clone());
        }

        // Phase 1 gate: every person completed Eat, Sleep, Work and a real
        // movement phase (no teleport). Idle is reported, not gated: under the
        // ADR-0018 default weights (Work 2300 vs Idle -50) a fully-reachable
        // Work/Meal/Rest fixture never selects Idle, so the run does not fail;
        // the Idle instrument is proven separately by a unit test.
        if args.gate && report.persons_completed_all_kinds != config.person_count {
            return Err(ChaosError::UnmetCompletion {
                detail: format!(
                    "{}/{} persons completed all four required observations (Eat/Sleep/Work + movement phase)",
                    report.persons_completed_all_kinds, config.person_count
                ),
            });
        }

        series.push(RunRow {
            index,
            wall_seconds: wall,
            sim_per_wall,
            events_per_wall,
            truth_hash: report.truth_hash,
            per_person_digest: report.per_person_digest,
            events_total: report.events_total,
            final_seconds: report.final_instant_seconds,
        });
    }

    let mut walls: Vec<f64> = series.iter().map(|row| row.wall_seconds).collect();
    walls.sort_by(f64::total_cmp);
    let median = walls[walls.len() / 2];
    let min = *walls.first().expect("runs exist");
    let max = *walls.last().expect("runs exist");

    let report = first_report.expect("at least one run");
    Ok((
        Output {
            config,
            runs: args.runs,
            min_wall_seconds: min,
            median_wall_seconds: median,
            max_wall_seconds: max,
            runs_series: series,
            determinism: args.runs >= 2,
        },
        report,
    ))
}

fn main() -> ExitCode {
    let args = match parse(std::env::args()) {
        Ok(args) => args,
        Err(message) if message.starts_with("HELP\n") => {
            println!("{}", message.trim_start_matches("HELP\n"));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let (output, report) = match run(&args) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("chaos_runner: {error}");
            return ExitCode::from(1);
        }
    };
    let mut report = report;
    let median = output.median_wall_seconds.max(1e-9);
    report.measurement = Some(ChaosMeasurement {
        wall_seconds: median,
        sim_seconds_per_wall: report.config.target_seconds().unwrap_or(0) as f64 / median,
        events_per_wall: report.events_total as f64 / median,
        peak_rss_delta_bytes: None,
    });
    let mut value = serde_json::to_value(&output).expect("output serializes");
    value["report"] = serde_json::to_value(&report).expect("report serializes");
    let text = match serde_json::to_string_pretty(&value) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("chaos_runner: serialization failed: {error}");
            return ExitCode::from(1);
        }
    };
    match std::fs::write(&args.out, text) {
        Ok(()) => eprintln!("chaos_runner: wrote {}", args.out),
        Err(error) => {
            eprintln!("chaos_runner: cannot write {}: {error}", args.out);
            return ExitCode::from(1);
        }
    }
    if let Some(path) = &args.markdown {
        let markdown = format!(
            "# Chaos Runner\n\n- Seed: {}\n- Persons: {}\n- Years: {}\n- Runs: {}\n- Median wall seconds: {:.3}\n- Deterministic: {}\n",
            output.config.seed,
            output.config.person_count,
            output.config.years,
            output.runs,
            output.median_wall_seconds,
            output.determinism
        );
        if let Err(error) = std::fs::write(path, markdown) {
            eprintln!("chaos_runner: cannot write {path}: {error}");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watchdog_returns_without_joining_a_stalled_worker() {
        let (release, wait) = mpsc::channel();
        let result = supervise(Duration::from_millis(20), move |heartbeat| {
            heartbeat();
            let _ = wait.recv();
            Ok(())
        });
        let _ = release.send(());
        assert!(matches!(
            result,
            Err(ChaosError::Invariant {
                rule: "watchdog",
                ..
            })
        ));
    }
    #[test]
    fn watchdog_reports_panics_and_passes_completed_work() {
        let panic = supervise::<()>(Duration::from_secs(2), |_| panic!("injected worker panic"));
        assert!(matches!(
            panic,
            Err(ChaosError::Invariant {
                rule: "worker panic",
                ..
            })
        ));
        assert_eq!(
            supervise(Duration::from_secs(2), |heartbeat| {
                heartbeat();
                Ok(7)
            })
            .unwrap(),
            7
        );
    }
    #[test]
    fn overflowing_horizon_is_rejected_without_running() {
        let args = Args {
            years: u64::MAX,
            ..Args::default()
        };
        assert!(matches!(config_for(&args), Err(ChaosError::Config(_))));
    }
}
