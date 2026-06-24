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
fn run_all_passing_fasta_assertions() {
    let output = exec(&["run", "tests/data/fasta_assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!(
        "run_fasta_passing_stdout",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn assert_count_passes() {
    let output = exec(&["assert", "tests/data/sample.fasta fasta.seq.count eq 3"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_total_length_passes() {
    let output = exec(&["assert", "tests/data/sample.fasta fasta.length eq 42"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_quoted_name_passes() {
    let output = exec(&[
        "assert",
        "tests/data/sample.fasta fasta.seq.2.name eq 'NC_000001.11'",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_count_zero_on_empty_fasta() {
    let output = exec(&["assert", "tests/data/empty.fasta fasta.seq.count eq 0"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_value_fails() {
    let output = exec(&[
        "assert",
        "tests/data/sample.fasta fasta.seq.0.name eq scaffold1",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn assert_errors_on_out_of_range_index() {
    let output = exec(&["assert", "tests/data/sample.fasta fasta.seq.3.name eq X"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR."),
        "expected ERROR. in stderr: {stderr}"
    );
    assert!(stderr.contains("record 3 not found"), "stderr: {stderr}");
}

#[test]
fn assert_errors_on_missing_description() {
    // chr2 (record 1) has no description, so a value check errors rather than failing.
    let output = exec(&[
        "assert",
        "tests/data/sample.fasta fasta.seq.1.description eq X",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn assert_errors_on_non_fasta_file() {
    // sample.bam is binary, so the FASTA reader fails to parse it as records.
    let output = exec(&["assert", "tests/data/sample.bam fasta.seq.count eq 1"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn present_is_false_for_missing_description_without_error() {
    // The .present check never errors on absence: chr2's missing description is reported false.
    let output = exec(&[
        "assert",
        "tests/data/sample.fasta fasta.seq.1.description.present eq false",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn present_is_false_for_out_of_range_record() {
    let output = exec(&[
        "assert",
        "tests/data/sample.fasta fasta.seq.3.present eq false",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}
