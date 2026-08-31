//! Independent cold-case RSS instrumentation. See ADR-0020 and README.md.
// Each standalone example owns its private protocol module; embedding the
// examples here intentionally includes those modules without exposing core API.
#![allow(clippy::duplicate_mod)]

mod rss;

// Reuse benchmark fixtures, assertions and new adapters; the timing mains are
// deliberately unused in this executable, not removed from their own examples.
#[allow(dead_code)]
#[path = "../../../crates/sim-core/examples/action_execution_bench.rs"]
mod action;
#[allow(dead_code)]
#[path = "../../../crates/sim-ai/examples/utility_ai_bench.rs"]
mod candidates;
#[allow(dead_code)]
#[path = "../../../crates/sim-world/examples/grid_bench.rs"]
mod grid;
#[allow(dead_code)]
#[path = "../../../crates/sim-core/examples/kernel_bench.rs"]
mod kernel;
#[allow(dead_code)]
#[path = "../../../crates/sim-ai/examples/needs_bench.rs"]
mod needs;
#[allow(dead_code)]
#[path = "../../../crates/sim-world/examples/pathfinding_bench.rs"]
mod pathfinding;
#[allow(dead_code)]
#[path = "../../../crates/sim-core/examples/person_spawn_bench.rs"]
mod person;
#[allow(dead_code)]
#[path = "../../../crates/sim-core/examples/render_snapshot_bench.rs"]
mod render;
#[allow(dead_code)]
#[path = "../../../crates/sim-world/examples/site_bench.rs"]
mod site;
#[allow(dead_code)]
#[path = "../../../crates/sim-ai/examples/utility_score_bench.rs"]
mod utility;
#[allow(dead_code)]
#[path = "../../../crates/sim-core/examples/worker_bench.rs"]
mod worker;
#[allow(dead_code)]
#[path = "../../../crates/sim-world/examples/worldgen_bench.rs"]
mod worldgen;

use std::hint::black_box;
use std::process::{Command, ExitCode};

use serde::Serialize;
use serde_json::{Value, json};

const CASES: &[&str] = &[
    "grid",
    "action-100",
    "action-1000",
    "worldgen-0",
    "worldgen-1",
    "worldgen-42",
    "person-100",
    "person-1000",
    "needs-100",
    "needs-1000",
    "sites",
    "path-trivial",
    "path-short",
    "path-medium",
    "path-long",
    "path-unreachable",
    "path-node_budget",
    "path-path_budget",
    "candidates-100",
    "candidates-1000",
    "utility-100-0",
    "utility-100-25",
    "utility-1000-0",
    "utility-1000-25",
    "kernel-100-year",
    "kernel-10-year",
    "render-control-100",
    "render-snapshot-100",
    "worker-100-day",
];
const PROBES: &[&str] = &[
    "probe-noop",
    "probe-retained",
    "probe-transient",
    "probe-contaminated",
    "probe-fail",
];

#[derive(Serialize)]
struct Sample<'a> {
    schema_version: u32,
    method: &'static str,
    case: &'a str,
    pid: u32,
    checksum: u64,
    cold: rss::Interval,
    operation: rss::Interval,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bench-memory: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [mode] if mode == "--list" => {
            println!(
                "{}",
                serde_json::to_string(CASES).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        [mode, case] if mode == "--child" => {
            validate_case(case)?;
            let sample = measure(case)?;
            println!(
                "{}",
                serde_json::to_string(&sample).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        [mode, case, count] if mode == "--run" => {
            if case != "all" {
                validate_case(case)?;
            }
            let count: usize = count
                .parse()
                .map_err(|_| "sample count must be an integer")?;
            if !(1..=100).contains(&count) {
                return Err("sample count must be in 1..=100".into());
            }
            let cases: Vec<&str> = if case == "all" {
                CASES.to_vec()
            } else {
                vec![case]
            };
            for case in cases {
                run_children(case, count)?;
            }
            Ok(())
        }
        _ => Err(
            "usage: palimpsest-bench-memory --list | --run <case|all> <1..100> | --child <case>"
                .into(),
        ),
    }
}

fn validate_case(case: &str) -> Result<(), String> {
    if CASES.contains(&case) || PROBES.contains(&case) {
        Ok(())
    } else {
        Err(format!("unknown memory case: {case}"))
    }
}

fn measure(case: &str) -> Result<Sample<'_>, String> {
    // A fixed-size observation buffer avoids heap allocation in callbacks.
    // Warm the instrumentation call itself, not the simulation fixture.
    rss::read()?;
    let mut observations = [None, None];
    let mut calls = 0;
    let mut failure = None;
    let mut observe = || {
        if calls >= observations.len() {
            failure = Some("adapter exceeded two observation boundaries".to_string());
            return;
        }
        match rss::read() {
            Ok(value) => observations[calls] = Some(value),
            Err(error) => failure = Some(error),
        }
        calls += 1;
    };
    let before = rss::read()?;
    let checksum = workload(case, &mut observe);
    if let Some(error) = failure {
        return Err(error);
    }
    if calls != 2 {
        return Err(format!("adapter observed {calls} boundaries, expected two"));
    }
    let prepared = observations[0].ok_or("missing prepared observation")?;
    let end = observations[1].ok_or("missing operation-end observation")?;
    let cold = rss::Interval::between(before, end)?;
    let operation = rss::Interval::between(prepared, end)?;
    if cold.peak_increment_bytes.is_none() {
        return Err(format!(
            "cold interval contaminated by earlier peak: {cold:?}"
        ));
    }
    Ok(Sample {
        schema_version: 1,
        method: "macos_kernel_rss_high_water_v1",
        case,
        pid: std::process::id(),
        checksum,
        cold,
        operation,
    })
}

fn workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    if let Some(case) = case.strip_prefix("action-") {
        action::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("kernel-") {
        kernel::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("render-") {
        render::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("worker-") {
        worker::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("worldgen-") {
        worldgen::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("person-") {
        person::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("needs-") {
        needs::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("path-") {
        pathfinding::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("candidates-") {
        candidates::memory_workload(case, observe)
    } else if let Some(case) = case.strip_prefix("utility-") {
        utility::memory_workload(case, observe)
    } else if case == "grid" {
        grid::memory_workload(case, observe)
    } else if case == "sites" {
        site::memory_workload(case, observe)
    } else {
        probe(case, observe)
    }
}

fn probe(case: &str, observe: &mut dyn FnMut()) -> u64 {
    match case {
        "probe-noop" => {
            observe();
            black_box(42);
            observe();
        }
        "probe-retained" => {
            observe();
            rss::probe_pages(true, observe).expect("mapped retained probe");
        }
        "probe-transient" => {
            observe();
            rss::probe_pages(false, observe).expect("mapped transient probe");
            observe();
        }
        "probe-contaminated" => {
            rss::probe_pages(false, observe).expect("mapped prior-peak probe");
            observe();
            black_box(42);
            observe();
        }
        "probe-fail" => panic!("intentional child failure probe"),
        _ => unreachable!("CLI validates the case before measuring"),
    }
    42
}

fn run_children(case: &str, count: usize) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut samples = Vec::with_capacity(count);
    for index in 0..count {
        let output = Command::new(&executable)
            .args(["--child", case])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "case {case}, sample {index} failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let mut sample: Value =
            serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        if sample["case"] != case || sample["cold"]["peak_increment_bytes"].as_u64().is_none() {
            return Err("invalid child case or missing proved cold peak".into());
        }
        if let Some(previous) = samples.first() {
            let previous: &Value = previous;
            if previous["checksum"] != sample["checksum"] {
                return Err(format!(
                    "nondeterministic checksum in {case}, sample {index}"
                ));
            }
        }
        sample["sample_index"] = json!(index);
        samples.push(sample);
    }
    let mut peaks: Vec<u64> = samples
        .iter()
        .map(|s| {
            s["cold"]["peak_increment_bytes"]
                .as_u64()
                .expect("validated child peak")
        })
        .collect();
    peaks.sort_unstable();
    println!(
        "{}",
        json!({"case":case, "samples":samples,
        "cold_peak_increment_min_bytes": peaks[0],
        "cold_peak_increment_median_bytes": peaks[count / 2],
        "cold_peak_increment_max_bytes": peaks[count - 1],
        "memory_warmups":0, "sampling":"fresh_process_per_sample"})
    );
    Ok(())
}
