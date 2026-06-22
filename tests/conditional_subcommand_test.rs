use insta::assert_snapshot;
use std::process::{Command, Output};

fn exec(args: &[&str]) -> Output {
    // disable console color so output assertions and snapshots match the plain text exactly
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .arg("--color=never")
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

#[test]
fn run_all_passing_or_skipped_exits_0() {
    let output = exec(&["run", "tests/data/conditional_assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!("run_conditional_stdout", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn guard_satisfied_runs_and_passes() {
    let output = exec(&["assert", "tests/data/example.tsv tsv.columns.count eq 3 if file.exists"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn guard_satisfied_runs_and_fails() {
    let output = exec(&["assert", "tests/data/example.tsv tsv.columns.count eq 99 if file.exists"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn guard_not_satisfied_skips_and_exits_0() {
    let output = exec(&["assert", "tests/data/missing.tsv tsv.columns.count eq 3 if file.exists"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("SKIP."));
}

#[test]
fn unless_skips_when_condition_holds() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.lines gt 0 unless file.empty"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("SKIP."));
}

#[test]
fn guard_that_errors_exits_2() {
    let output = exec(&["assert", "tests/data/example.tsv tsv.lines.count gt 0 if tests/data/missing.tsv file.size gt 0B"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("guard could not be evaluated"), "expected guard error message in stderr: {stderr}");
}

#[test]
fn shorthand_guard_on_a_numeric_metric_errors() {
    let output = exec(&["assert", "tests/data/example.tsv tsv.lines.count gt 0 if tsv.columns.count"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}
