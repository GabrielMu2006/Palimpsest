#[path = "../examples/support/bench_protocol.rs"]
mod protocol;

#[test]
fn strict_flags_reject_unknown_duplicate_missing_and_zero_samples() {
    for args in [
        vec!["--unknown"],
        vec!["--samples"],
        vec!["--samples", "0"],
        vec!["--samples", "2", "--samples", "3"],
        vec!["--persons", "oops"],
    ] {
        assert!(protocol::parse(args, protocol::defaults()).is_err());
    }
}

#[test]
fn upper_median_and_submicro_precision_are_preserved() {
    let mut even = [9_u128, 1, 7, 3];
    assert_eq!(protocol::median(&mut even), 7);
    let mut precise = [1_u128, 2, 3];
    assert_eq!(protocol::median(&mut precise), 2);
}

#[test]
fn per_tool_flags_cannot_be_silently_ignored() {
    for args in [vec!["--seed", "7"], vec!["--seconds", "12"]] {
        assert!(
            protocol::parse_for(
                args,
                protocol::defaults(),
                &["--persons", "--samples", "--warmups", "--json"]
            )
            .is_err()
        );
    }
    for args in [
        vec!["--seconds", "0"],
        vec!["--seconds", "-1"],
        vec!["--warmups", "1.2"],
        vec!["--json", "--json"],
    ] {
        assert!(protocol::parse(args, protocol::defaults()).is_err());
    }
    assert!(protocol::parse(["--warmups", "0", "--persons", "0"], protocol::defaults()).is_ok());
}
