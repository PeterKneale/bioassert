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
fn run_all_passing_bam_assertions() {
    let output = exec(&["run", "tests/data/bam_assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!(
        "run_bam_passing_stdout",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn assert_count_passes() {
    let output = exec(&["assert", "tests/data/sample.bam bam.header.rg.count eq 2"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_value_fails() {
    let output = exec(&[
        "assert",
        "tests/data/sample.bam bam.header.rg.0.sm eq WRONG",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn assert_errors_on_out_of_range_index() {
    let output = exec(&["assert", "tests/data/sample.bam bam.header.rg.2.sm eq X"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR."),
        "expected ERROR. in stderr: {stderr}"
    );
    assert!(
        stderr.contains("read group 2 tag sm not found"),
        "stderr: {stderr}"
    );
}

#[test]
fn assert_errors_on_missing_tag() {
    let output = exec(&["assert", "tests/data/sample.bam bam.header.rg.0.dt eq X"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn assert_errors_on_non_bam_file() {
    let output = exec(&[
        "assert",
        "tests/data/empty_file.txt bam.header.rg.count eq 1",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn present_is_false_for_missing_tag_without_error() {
    // The .present check never errors on absence: a missing DT tag is reported as false.
    let output = exec(&[
        "assert",
        "tests/data/sample.bam bam.header.rg.0.dt.present eq false",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_program_count_uppercase_namespace_passes() {
    // The `pg` segment is case-insensitive: `PG` matches `pg` (@PG is uppercase in the file).
    let output = exec(&["assert", "tests/data/sample.bam bam.header.PG.count eq 2"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_program_chaining_passes() {
    // The second program chains from the first via its PP (previous-program) tag.
    let output = exec(&["assert", "tests/data/sample.bam bam.header.pg.1.pp eq bwa"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_errors_on_out_of_range_program() {
    let output = exec(&["assert", "tests/data/sample.bam bam.header.pg.5.id eq X"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR."),
        "expected ERROR. in stderr: {stderr}"
    );
    assert!(
        stderr.contains("program 5 tag id not found"),
        "stderr: {stderr}"
    );
}
