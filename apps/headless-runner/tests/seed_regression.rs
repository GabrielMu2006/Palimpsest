//! Reviewed deterministic corpus; these tests never rewrite expectations.
use palimpsest_sim_core::{ChaosConfig, run_chaos};
use serde_json::Value;
fn assert_expected(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err("deterministic corpus changed: approved behavior change and reviewed old/new diff required".into())
    }
}
#[test]
fn reviewed_seed_corpus_reproduces_complete_reports_twice() {
    for seed in [0, 1, 42] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../tests/worlds/seed_{seed}.json"));
        let fixture: Value =
            serde_json::from_slice(&std::fs::read(path).expect("reviewed corpus file")).unwrap();
        let config: ChaosConfig = serde_json::from_value(fixture["config"].clone()).unwrap();
        let mut first = None;
        for _ in 0..2 {
            let report = run_chaos(&config, true).expect("real chaos smoke");
            assert_eq!(report.persons_completed_all_kinds, config.person_count);
            assert!(report.violated_invariants.is_empty());
            let actual = serde_json::to_value(&report).unwrap();
            assert_expected(&actual, &fixture["expected"])
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            if let Some(previous) = &first {
                assert_eq!(previous, &actual);
            } else {
                first = Some(actual.clone());
            }
            // Deliberately broken expectation proves the gate, without changing disk files.
            let mut corrupt = actual.clone();
            corrupt["truth_hash"] = serde_json::json!(0);
            assert!(assert_expected(&actual, &corrupt).is_err());
        }
    }
}

#[test]
fn broken_world_configuration_fails_before_a_golden_can_pass() {
    let invalid = ChaosConfig {
        seed: 42,
        person_count: 0,
        years: 1,
        sim_seconds_per_year: 86400,
    };
    assert!(run_chaos(&invalid, true).is_err());
}
