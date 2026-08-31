use std::process::{Command, Output};

#[cfg(target_os = "macos")]
use serde_json::Value;

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_palimpsest-bench-memory"))
        .args(arguments)
        .output()
        .expect("launch memory tool")
}

#[test]
fn list_has_exactly_the_29_unique_planned_cases() {
    let output = invoke(&["--list"]);
    assert!(output.status.success());
    let cases: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(cases.len(), 29);
    let unique: std::collections::BTreeSet<_> = cases.iter().collect();
    assert_eq!(unique.len(), cases.len());
    for expected in [
        "grid",
        "action-100",
        "action-1000",
        "worldgen-42",
        "person-1000",
        "needs-1000",
        "sites",
        "path-unreachable",
        "path-node_budget",
        "candidates-1000",
        "utility-1000-25",
        "kernel-100-year",
        "render-control-100",
        "render-snapshot-100",
        "worker-100-day",
    ] {
        assert!(unique.contains(&expected.to_string()));
    }
}

#[test]
fn invalid_case_count_or_cli_fails_without_measurement_output() {
    for arguments in [
        vec![],
        vec!["--child", "unknown"],
        vec!["--run", "grid", "0"],
        vec!["--run", "grid", "101"],
        vec!["--run", "grid", "-1"],
        vec!["--run", "grid", "1.5"],
        vec!["--run", "all", "not-a-number"],
        vec!["--run", "grid", "1", "extra"],
    ] {
        let output = invoke(&arguments);
        assert!(!output.status.success(), "accepted invalid {arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn unsupported_platform_returns_error_not_zero_rss() {
    let output = invoke(&["--child", "probe-noop"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("macOS only"));
}

#[cfg(target_os = "macos")]
fn child(case: &str) -> Value {
    let output = invoke(&["--child", case]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["case"], case);
    assert_eq!(value["method"], "macos_kernel_rss_high_water_v1");
    value
}

#[cfg(target_os = "macos")]
fn bytes(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap()
}

#[cfg(target_os = "macos")]
#[test]
fn kernel_peak_captures_64mib_released_between_endpoints() {
    let sample = child("probe-transient");
    let peak = bytes(&sample, "/operation/end/lifetime_peak_bytes");
    let baseline = bytes(&sample, "/operation/baseline/current_bytes");
    let end = bytes(&sample, "/operation/end/current_bytes");
    assert!(peak - baseline >= 60 * 1024 * 1024);
    // An endpoint-only implementation cannot pass this regression.
    assert!(peak - end >= 48 * 1024 * 1024);
    assert_eq!(
        bytes(&sample, "/operation/peak_increment_bytes"),
        peak - baseline
    );
}

#[cfg(target_os = "macos")]
#[test]
fn retained_pages_use_byte_units_and_remain_resident_at_end() {
    let sample = child("probe-retained");
    let baseline = bytes(&sample, "/operation/baseline/current_bytes");
    let end = bytes(&sample, "/operation/end/current_bytes");
    let delta = bytes(&sample, "/operation/peak_increment_bytes");
    assert!(end - baseline >= 60 * 1024 * 1024);
    assert!((60 * 1024 * 1024..80 * 1024 * 1024).contains(&delta));
}

#[cfg(target_os = "macos")]
#[test]
fn prior_peak_marks_prepared_interval_ambiguous() {
    let sample = child("probe-contaminated");
    assert!(bytes(&sample, "/cold/peak_increment_bytes") >= 60 * 1024 * 1024);
    assert_eq!(sample["operation"]["proof"], "ambiguous_prior_peak");
    assert!(sample["operation"]["peak_increment_bytes"].is_null());
}

#[cfg(target_os = "macos")]
#[test]
fn each_sample_is_a_new_process_and_a_previous_peak_does_not_leak() {
    let large = child("probe-retained");
    let output = invoke(&["--run", "probe-noop", "3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    let samples = result["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 3);
    let mut pids = std::collections::BTreeSet::new();
    for (index, sample) in samples.iter().enumerate() {
        assert!(pids.insert(sample["pid"].as_u64().unwrap()));
        assert_ne!(sample["pid"], large["pid"]);
        assert_eq!(sample["checksum"], 42);
        assert_eq!(sample["sample_index"], index);
        assert!(bytes(sample, "/cold/peak_increment_bytes") < 1024 * 1024);
        assert!(
            bytes(sample, "/cold/baseline/lifetime_peak_bytes")
                < bytes(&large, "/cold/end/lifetime_peak_bytes")
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn child_failure_is_not_retried_or_turned_into_a_success_row() {
    let output = invoke(&["--run", "probe-fail", "3"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("sample 0 failed"));
    assert!(error.contains("intentional child failure probe"));
}

#[cfg(target_os = "macos")]
#[test]
fn render_selectors_report_the_same_prepared_truth_and_two_intervals() {
    let control = child("render-control-100");
    let snapshot = child("render-snapshot-100");
    assert_eq!(control["checksum"], snapshot["checksum"]);
    assert_ne!(control["pid"], snapshot["pid"]);
    for sample in [control, snapshot] {
        for interval in ["cold", "operation"] {
            assert!(
                sample[interval]["baseline"]["current_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert!(sample[interval]["end"]["current_bytes"].as_u64().unwrap() > 0);
            assert!(sample[interval]["proof"].is_string());
        }
    }
}
