//! Single cold-process native RSS for the representative CHRON-033 fixture.
#[allow(dead_code)]
#[path = "../rss.rs"]
mod rss;
use palimpsest_headless_runner::micro_bench::{advance_to_target, build_fixture, validate_kernel};
use serde_json::json;
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let persons = match args.as_slice() {
        [value] => value.parse::<usize>().map_err(|e| e.to_string())?,
        _ => return Err("usage: micro_memory positive-persons".into()),
    };
    rss::read()?;
    let baseline = rss::read()?;
    let mut kernel = build_fixture(persons, 86_400)?;
    let prepared = rss::read()?;
    advance_to_target(&mut kernel, 86_400)?;
    validate_kernel(&mut kernel, persons, 86_400)?;
    let snapshot =
        palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let hash = snapshot.diagnostic_hash();
    let end = rss::read()?;
    let cold = rss::Interval::between(baseline, end)?;
    let operation = rss::Interval::between(prepared, end)?;
    println!(
        "{}",
        json!({"schema_version":1,"method":"macos_kernel_rss_high_water_v1",
        "samples":1,"persons":persons,"seconds":86400,"cold":cold,"operation":operation,
        "snapshot_bytes":bytes.len(),"snapshot_hash":hash.to_string(),
        "interval":"setup+advance+validation+snapshot serialization; snapshot and bytes retained"})
    );
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("micro_memory: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
