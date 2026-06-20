use std::process::{Command, Output};

fn exec(args: &[&str]) -> Output {
    // disable console color so output assertions match the plain text exactly
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .arg("--color=never")
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

#[test]
fn exits_0_on_assertion_pass() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.exists eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn exits_1_on_assertion_failure() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.lines gt 999"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn exits_1_for_missing_file() {
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.exists eq true"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn exits_2_on_error() {
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.size gt 0B"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn exits_2_for_metric_error() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.explode eq 0"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("unknown metric"), "expected 'unknown metric' in stderr: {stderr}");
}

#[test]
fn exits_2_for_comparator_error() {
    let output = exec(&["assert", "tests/data/example.csv csv.line.1.column.1 lt Alice"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("unsupported comparator"), "expected 'unsupported comparator' in stderr: {stderr}");
}

#[test]
fn exits_2_for_value_error() {
    let output = exec(&["assert", "tests/data/empty_file.txt file.lines eq notanumber"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("Invalid integer"), "expected 'Invalid integer' in stderr: {stderr}");
}

#[test]
fn exits_2_for_file_error() {
    let output = exec(&["assert", "tests/data/nonexistent_file.txt file.size eq 0B"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("nonexistent_file.txt"), "expected path in stderr: {stderr}");
}

#[test]
fn exits_2_for_regex_error() {
    let output = exec(&["assert", "tests/data/example.csv csv.line.1.column.1 matches '[invalid'"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "expected ERROR. in stderr: {stderr}");
    assert!(stderr.contains("invalid regex"), "expected 'invalid regex' in stderr: {stderr}");
}
