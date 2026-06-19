use insta::assert_snapshot;
use std::process::{Command, Output};

fn exec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

#[test]
fn exits_0_for_all_passing_assertions() {
    let output = exec(&["run", "tests/data/assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );
    assert_snapshot!("run_all_passing_stdout", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn exits_1_when_assertion_fails() {
    let output = exec(&["run", "tests/data/failing_assertions.txt"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("PASS."), "expected at least one PASS line");
    assert!(stdout.contains("FAIL."), "expected at least one FAIL line");
}

#[test]
fn fail_output_snapshot() {
    let output = exec(&["run", "tests/data/failing_assertions.txt"]);
    assert_eq!(output.status.code(), Some(1));
    assert_snapshot!("run_fail_stdout", String::from_utf8_lossy(&output.stdout));
    assert!(output.stderr.is_empty());
}

#[test]
fn exits_2_for_invalid_metric() {
    let output = exec(&["run", "tests/data/invalid_metric.txt"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn error_output_snapshot() {
    let output = exec(&["run", "tests/data/invalid_metric.txt"]);
    assert_eq!(output.status.code(), Some(2));
    assert_snapshot!("run_error_stdout", String::from_utf8_lossy(&output.stdout));
    assert_snapshot!("run_error_stderr", String::from_utf8_lossy(&output.stderr));
}
