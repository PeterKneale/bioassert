use std::process::{Command, Output};

fn bioassert(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

// assert subcommand

#[test]
fn assert_exits_0_on_assertion_pass() {
    let output = bioassert(&["assert", "tests/data/empty_file.txt file.exists eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_exits_1_on_assertion_failure() {
    let output = bioassert(&["assert", "tests/data/empty_file.txt file.lines gt 999"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn assert_exits_1_for_missing_file() {
    // file.exists returns false (not an error) for a nonexistent file — exit 1
    let output = bioassert(&["assert", "tests/data/nonexistent_file.txt file.exists eq true"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn assert_exits_2_on_error() {
    // file.size on a nonexistent file is a runtime error — exit 2
    let output = bioassert(&["assert", "tests/data/nonexistent_file.txt file.size gt 0B"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

// run subcommand

#[test]
fn run_exits_1_when_assertion_fails() {
    let output = bioassert(&["run", "tests/data/failing_assertions.txt"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("PASS."), "expected at least one PASS line");
    assert!(stdout.contains("FAIL."), "expected at least one FAIL line");
}

#[test]
fn run_exits_2_for_invalid_metric() {
    let output = bioassert(&["run", "tests/data/invalid_metric.txt"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn run_exits_0_for_all_passing_assertions() {
    let output = bioassert(&["run", "tests/data/assertions.txt"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}",
        output.status,
        stdout
    );
    assert_eq!(stdout.trim_end(), EXPECTED_OUTPUT);
}

const EXPECTED_OUTPUT: &str = "\
Running assertions in tests/data/assertions.txt
PASS. Expected tests/data/empty_file.txt file.exists == true, got true
PASS. Expected tests/data/empty_file.txt file.exists != false, got true
PASS. Expected tests/data/size_5B.txt file.exists == true, got true
PASS. Expected tests/data/size_1K.txt file.exists == true, got true
PASS. Expected tests/data/empty_file.txt file.size == 0B, got 0B
PASS. Expected tests/data/size_5B.txt file.size == 5B, got 5B
PASS. Expected tests/data/size_5B.txt file.size == 5B, got 5B
PASS. Expected tests/data/size_1K.txt file.size == 1.00KB, got 1.00KB
PASS. Expected tests/data/size_1K.txt file.size == 1.00KB, got 1.00KB
PASS. Expected tests/data/empty_file.txt file.size != 1B, got 0B
PASS. Expected tests/data/size_5B.txt file.size != 4B, got 5B
PASS. Expected tests/data/size_1K.txt file.size != 1023B, got 1.00KB
PASS. Expected tests/data/empty_file.txt file.size < 1B, got 0B
PASS. Expected tests/data/size_5B.txt file.size < 6B, got 5B
PASS. Expected tests/data/size_1K.txt file.size < 2.00KB, got 1.00KB
PASS. Expected tests/data/size_5B.txt file.size <= 5B, got 5B
PASS. Expected tests/data/size_5B.txt file.size <= 6B, got 5B
PASS. Expected tests/data/size_1K.txt file.size <= 1.00KB, got 1.00KB
PASS. Expected tests/data/size_5B.txt file.size > 4B, got 5B
PASS. Expected tests/data/size_1K.txt file.size > 500B, got 1.00KB
PASS. Expected tests/data/size_5B.txt file.size >= 5B, got 5B
PASS. Expected tests/data/size_5B.txt file.size >= 4B, got 5B
PASS. Expected tests/data/size_1K.txt file.size >= 1.00KB, got 1.00KB
PASS. Expected tests/data/empty_file.txt file.empty == true, got true
PASS. Expected tests/data/empty_file.txt file.empty != false, got true
PASS. Expected tests/data/size_5B.txt file.empty == false, got false
PASS. Expected tests/data/size_5B.txt file.empty != true, got false
PASS. Expected tests/data/size_1K.txt file.empty == false, got false
PASS. Expected tests/data/empty_file.txt file.lines == 0, got 0
PASS. Expected tests/data/lines_1.txt file.lines == 1, got 1
PASS. Expected tests/data/lines_10.txt file.lines == 10, got 10
PASS. Expected tests/data/lines_100.txt file.lines == 100, got 100
PASS. Expected tests/data/lines_10.txt file.lines != 9, got 10
PASS. Expected tests/data/lines_10.txt file.lines != 11, got 10
PASS. Expected tests/data/lines_10.txt file.lines < 11, got 10
PASS. Expected tests/data/lines_100.txt file.lines < 101, got 100
PASS. Expected tests/data/lines_10.txt file.lines <= 10, got 10
PASS. Expected tests/data/lines_10.txt file.lines <= 11, got 10
PASS. Expected tests/data/lines_10.txt file.lines > 9, got 10
PASS. Expected tests/data/lines_100.txt file.lines > 99, got 100
PASS. Expected tests/data/lines_10.txt file.lines >= 10, got 10
PASS. Expected tests/data/lines_10.txt file.lines >= 9, got 10
PASS. Expected tests/data/example.csv csv.columns.count > 2, got 3
PASS. Expected tests/data/example.csv csv.columns.count == 3, got 3
PASS. Expected tests/data/example.csv csv.columns.count < 4, got 3
PASS. Expected tests/data/example.csv csv.lines.count > 2, got 3
PASS. Expected tests/data/example.csv csv.lines.count == 3, got 3
PASS. Expected tests/data/example.csv csv.lines.count < 4, got 3
PASS. Expected tests/data/example.csv csv.line.1.column.1 == name, got name
PASS. Expected tests/data/example.csv csv.line.1.column.2 == age, got age
PASS. Expected tests/data/example.csv csv.line.1.column.3 == city, got city
PASS. Expected tests/data/example.csv csv.line.2.column.1 == Alice, got Alice
PASS. Expected tests/data/example.csv csv.line.3.column.3 == Los Angeles, got Los Angeles
PASS. Expected tests/data/example.csv csv.line.2.column.1 starts_with A, got Alice
PASS. Expected tests/data/example.csv csv.line.2.column.1 contains lic, got Alice
PASS. Expected tests/data/example.csv csv.line.2.column.1 ends_with e, got Alice
PASS. Expected tests/data/example.tsv tsv.columns.count > 2, got 3
PASS. Expected tests/data/example.tsv tsv.columns.count == 3, got 3
PASS. Expected tests/data/example.tsv tsv.columns.count < 4, got 3
PASS. Expected tests/data/example.tsv tsv.lines.count > 2, got 3
PASS. Expected tests/data/example.tsv tsv.lines.count == 3, got 3
PASS. Expected tests/data/example.tsv tsv.lines.count < 4, got 3
PASS. Expected tests/data/example.tsv tsv.line.1.column.1 == name, got name
PASS. Expected tests/data/example.tsv tsv.line.1.column.2 == age, got age
PASS. Expected tests/data/example.tsv tsv.line.1.column.3 == city, got city
PASS. Expected tests/data/example.tsv tsv.line.2.column.1 == Alice, got Alice
PASS. Expected tests/data/example.tsv tsv.line.3.column.3 == Los Angeles, got Los Angeles
PASS. Expected tests/data/example.tsv tsv.line.2.column.1 starts_with A, got Alice
PASS. Expected tests/data/example.tsv tsv.line.2.column.1 contains lic, got Alice
PASS. Expected tests/data/example.tsv tsv.line.2.column.1 ends_with e, got Alice
PASS. Expected tests/data/example.psv psv.columns.count > 2, got 3
PASS. Expected tests/data/example.psv psv.columns.count == 3, got 3
PASS. Expected tests/data/example.psv psv.columns.count < 4, got 3
PASS. Expected tests/data/example.psv psv.lines.count > 2, got 3
PASS. Expected tests/data/example.psv psv.lines.count == 3, got 3
PASS. Expected tests/data/example.psv psv.lines.count < 4, got 3
PASS. Expected tests/data/example.psv psv.line.1.column.1 == name, got name
PASS. Expected tests/data/example.psv psv.line.1.column.2 == age, got age
PASS. Expected tests/data/example.psv psv.line.1.column.3 == city, got city
PASS. Expected tests/data/example.psv psv.line.2.column.1 == Alice, got Alice
PASS. Expected tests/data/example.psv psv.line.3.column.3 == Los Angeles, got Los Angeles
PASS. Expected tests/data/example.psv psv.line.2.column.1 starts_with A, got Alice
PASS. Expected tests/data/example.psv psv.line.2.column.1 contains lic, got Alice
PASS. Expected tests/data/example.psv psv.line.2.column.1 ends_with e, got Alice";
