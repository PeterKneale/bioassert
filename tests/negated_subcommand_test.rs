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
fn run_all_passing_exits_0() {
    let output = exec(&["run", "tests/data/negated_comparators.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!(
        "run_negated_stdout",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn file_contents_absence_passes() {
    // The motivating case: a clean log has no Exception anywhere (replaces grep -v).
    let output = exec(&[
        "assert",
        "tests/data/clean_log.txt file.contents not contains 'Exception'",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn file_contents_presence_of_absent_word_fails() {
    let output = exec(&[
        "assert",
        "tests/data/clean_log.txt file.contents contains 'Exception'",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn file_contents_negation_of_present_word_fails() {
    // `not contains completed` fails because the word IS present
    let output = exec(&[
        "assert",
        "tests/data/clean_log.txt file.contents not contains completed",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn column_none_semantics_reports_first_offending_cell() {
    // `*.column.N.data.all not contains N` means "no data cell contains N"; the anchor
    // column has NDA on the file's line 3, so it fails and names that line.
    let output = exec(&[
        "assert",
        "tests/data/junctions.tsv tsv.column.11.data.all not contains N",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FAIL."), "stdout: {stdout}");
    assert!(stdout.contains("line 3"), "stdout: {stdout}");
}

#[test]
fn column_none_semantics_passes_when_no_cell_matches() {
    // strand cells are + or -, so none contains N
    let output = exec(&[
        "assert",
        "tests/data/junctions.tsv tsv.column.6.data.all not contains N",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn not_with_a_numeric_comparator_is_accepted() {
    // `not gt 1MB` equals `lte 1MB`; a 5-byte file is not greater than 1MB
    let output = exec(&["assert", "tests/data/size_5B.txt file.size not gt 1MB"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn file_contents_on_a_binary_file_errors() {
    // A non-UTF-8 file handed to a text metric reports ERROR (exit 2), not a silent pass.
    let output = exec(&["assert", "tests/data/sample.bam file.contents contains x"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}
