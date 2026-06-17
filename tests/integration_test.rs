use insta::assert_snapshot;
use std::process::{Command, Output};

fn exec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

// assert subcommand

#[test]
fn assert_exits_0_on_assertion_pass() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.exists eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_exits_1_on_assertion_failure() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.lines gt 999"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn assert_exits_1_for_missing_file() {
    // file.exists returns false (not an error) for a nonexistent file — exit 1
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.exists eq true"]);
    assert_exit_code_failed(output);
}


#[test]
fn assert_exits_2_on_error() {
    // file.size on a nonexistent file is a runtime error — exit 2
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.size gt 0B"]);
    assert_exit_code_is_error(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

fn assert_exit_code_failed(output: Output) {
    assert_eq!(output.status.code(), Some(1));
}
fn assert_exit_code_is_error(output: &Output) {
    assert_eq!(output.status.code(), Some(2));
}

// BioAssertError variants — one test per variant, all using the inline assert subcommand

#[test]
fn assert_exits_2_for_metric_error() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.explode eq 0"]);
    assert_exit_code_is_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("unknown metric"), "expected 'unknown metric' in stderr: {stderr}");
}

#[test]
fn assert_exits_2_for_comparator_error() {
    // numeric comparator (lt) on a cell assertion triggers UnsupportedComparator at runtime
    let output = exec(&["assert", "tests/data/example.csv csv.line.1.column.1 lt Alice"]);
    assert_exit_code_is_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("unsupported comparator"), "expected 'unsupported comparator' in stderr: {stderr}");
}

#[test]
fn assert_exits_2_for_value_error() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.lines eq notanumber"]);
    assert_exit_code_is_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("Invalid integer"), "expected 'Invalid integer' in stderr: {stderr}");
}

#[test]
fn assert_exits_2_for_file_error() {
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.size eq 0B"]);
    assert_exit_code_is_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("nonexistent_file.txt"), "expected path in stderr: {stderr}");
}

#[test]
fn assert_exits_2_for_regex_error() {
    // single-quoted value passes the grammar; strip_quotes removes quotes before regex compilation fails
    let output = exec(&["assert", "tests/data/example.csv csv.line.1.column.1 matches '[invalid'"]);
    assert_exit_code_is_error(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("invalid regex"), "expected 'invalid regex' in stderr: {stderr}");
}

// run subcommand

#[test]
fn run_exits_1_when_assertion_fails() {
    let output = exec(&["run", "tests/data/failing_assertions.txt"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("PASS."), "expected at least one PASS line");
    assert!(stdout.contains("FAIL."), "expected at least one FAIL line");
}

#[test]
fn run_fail_output_snapshot() {
    let output = exec(&["run", "tests/data/failing_assertions.txt"]);
    assert_eq!(output.status.code(), Some(1));
    assert_snapshot!("run_fail_stdout", String::from_utf8_lossy(&output.stdout));
    assert!(output.stderr.is_empty());
}

#[test]
fn run_exits_2_for_invalid_metric() {
    let output = exec(&["run", "tests/data/invalid_metric.txt"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn run_error_output_snapshot() {
    let output = exec(&["run", "tests/data/invalid_metric.txt"]);
    assert_eq!(output.status.code(), Some(2));
    assert_snapshot!("run_error_stdout", String::from_utf8_lossy(&output.stdout));
    assert_snapshot!("run_error_stderr", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn run_exits_0_for_all_passing_assertions() {
    let output = exec(&["run", "tests/data/assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );
    assert_snapshot!("run_all_passing_stdout", String::from_utf8_lossy(&output.stdout));
}
