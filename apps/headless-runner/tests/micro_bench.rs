use palimpsest_headless_runner::micro_bench;

#[test]
fn fixture_smoke_advances_and_validates() {
    let mut kernel = micro_bench::build_fixture(2, 86_400).expect("fixture");
    micro_bench::advance_to_target(&mut kernel, 86_400).expect("advance");
    micro_bench::validate_kernel(&mut kernel, 2, 86_400).expect("validate");
}

#[test]
fn invalid_fixture_inputs_are_rejected() {
    assert!(micro_bench::build_fixture(0, 86_400).is_err());
    assert!(micro_bench::build_fixture(1, 0).is_err());
}

#[test]
fn representative_measurement_counts_real_work_and_snapshot() {
    let row = micro_bench::measure_scale(2, 86_400).expect("complete sample");
    assert!(
        row["work"]["candidate_queries"].as_u64().unwrap()
            > row["work"]["decisions"].as_u64().unwrap()
    );
    assert!(row["work"]["scheduler_dequeued"].as_u64().unwrap() > 0);
    assert_eq!(row["path_probe"]["successes"], 2);
    assert!(row["snapshot_bytes"].as_u64().unwrap() > 16_384);
}
#[test]
fn benchmark_cli_rejects_bad_configuration() {
    for args in [
        vec!["--samples", "0"],
        vec!["--scales", "0"],
        vec!["--scales", "2,2"],
        vec!["--seconds", "-1"],
        vec!["--unknown", "1"],
        vec!["--warmups", "0", "--warmups", "0"],
    ] {
        assert!(
            !std::process::Command::new(env!("CARGO_BIN_EXE_bench_micro_world"))
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
