use palimpsest_headless_runner::{RunError, run};
#[test]
fn real_runner_reaches_a_day_and_preserves_live_future_work() {
    let metrics = run(100, 86400).unwrap();
    assert_eq!(metrics.entities, 100);
    assert_eq!(metrics.final_sim_second, 86400);
    assert!(metrics.processed_work > metrics.generated_events && metrics.generated_events > 0);
    assert_eq!(metrics.remaining_scheduled, 200);
    assert_eq!(metrics.snapshot_hash, "14346005809762790435");
}
#[test]
fn real_runner_rejects_negative_time_and_zero_population() {
    assert_eq!(run(1, -1), Err(RunError::NegativeFinalTime(-1)));
    assert_eq!(run(0, 1), Err(RunError::InvalidPopulation(0)));
    let initial = run(1, 0).unwrap();
    assert_eq!(initial.final_sim_second, 0);
    assert_eq!(initial.generated_events, 0);
    assert!(initial.remaining_scheduled > 0);
}
