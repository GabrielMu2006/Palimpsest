use std::process::Command;

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_chaos_runner"))
}

#[test]
fn help_and_invalid_inputs_fail_fast() {
    assert!(
        runner()
            .arg("--help")
            .status()
            .expect("runner starts")
            .success()
    );
    assert!(
        !runner()
            .args(["--persons", "0"])
            .status()
            .expect("runner starts")
            .success()
    );
    assert!(
        !runner()
            .args(["--years", "0"])
            .status()
            .expect("runner starts")
            .success()
    );
    assert!(
        !runner()
            .args(["--runs", "0"])
            .status()
            .expect("runner starts")
            .success()
    );
    assert!(
        !runner()
            .args(["--seconds", "0"])
            .status()
            .expect("runner starts")
            .success()
    );
    assert!(
        !runner()
            .args(["--watchdog-seconds", "0"])
            .status()
            .expect("runner starts")
            .success()
    );
}
