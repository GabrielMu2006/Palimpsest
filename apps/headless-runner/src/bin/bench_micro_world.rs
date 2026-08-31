//! Sequential representative measurements. Failures preserve earlier raw samples.
#![allow(clippy::cast_precision_loss)]
use palimpsest_headless_runner::micro_bench::measure_scale;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn same(a: &Value, b: &Value) -> bool {
    ["snapshot_hash", "work", "events_digest", "queue"]
        .iter()
        .all(|key| a[key] == b[key])
}
fn summary(rows: &[Value]) -> Value {
    let mut out = serde_json::Map::new();
    for key in ["elapsed_ns", "snapshot_build_ns", "snapshot_serialize_ns"] {
        let mut v: Vec<f64> = rows
            .iter()
            .map(|x| x[key].as_u64().expect("nanoseconds") as f64)
            .collect();
        v.sort_by(f64::total_cmp);
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        out.insert(key.into(),json!({"median":v[v.len()/2],"min":v[0],"max":v[v.len()-1],"mean":mean,"variance":v.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/v.len()as f64}));
    }
    Value::Object(out)
}
fn run() -> Result<(), String> {
    let mut scales = vec![100, 1000, 3000, 5000, 10000];
    let mut seconds = 86400_i64;
    let mut samples = 10_usize;
    let mut warmups = 2_usize;
    let mut seen = BTreeSet::new();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!(
                "bench_micro_world --scales comma-list --seconds >=86400 --samples positive --warmups nonnegative"
            );
            return Ok(());
        }
        if !seen.insert(flag.clone()) {
            return Err(format!("duplicate option {flag}"));
        }
        let value = args.next().ok_or_else(|| format!("missing value {flag}"))?;
        match flag.as_str() {
            "--scales" => {
                scales = value
                    .split(',')
                    .map(str::parse)
                    .collect::<Result<Vec<usize>, _>>()
                    .map_err(|e| e.to_string())?;
            }
            "--seconds" => {
                seconds = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--samples" => {
                samples = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            "--warmups" => {
                warmups = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?;
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if seconds < 86400
        || samples == 0
        || scales.is_empty()
        || scales.contains(&0)
        || scales.iter().collect::<BTreeSet<_>>().len() != scales.len()
    {
        return Err("invalid horizon/sample/scale configuration".into());
    }
    let mut failed = false;
    for scale in scales {
        let mut rows = Vec::new();
        let mut warm = Vec::new();
        let mut failure = None;
        for index in 0..warmups
            .checked_add(samples)
            .ok_or("sample count overflow")?
        {
            match measure_scale(scale, seconds) {
                Ok(row) => {
                    let reference = warm.first().or_else(|| rows.first());
                    if reference.is_some_and(|r| !same(r, &row)) {
                        failure = Some("deterministic work differs".to_string());
                    }
                    if index < warmups {
                        warm.push(row);
                    } else {
                        rows.push(row);
                    }
                }
                Err(error) => failure = Some(error),
            }
            if failure.is_some() {
                break;
            }
        }
        let stats = if rows.is_empty() {
            Value::Null
        } else {
            summary(&rows)
        };
        println!(
            "{}",
            json!({"schema_version":1,"scale":scale,"seconds":seconds,"requested_warmups":warmups,"requested_samples":samples,
           "profile":if cfg!(debug_assertions){"debug"}else{"release"},"status":if failure.is_some(){"failed"}else{"passed"},"error":failure,
           "warmups":warm,"samples":rows,"summary":stats,"protocol":"2 warmups/10 samples formal; overrides are smoke only; upper median, population variance"})
        );
        failed |= failure.is_some();
    }
    if failed {
        Err("one or more scales failed; raw results retained".into())
    } else {
        Ok(())
    }
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bench_micro_world: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
