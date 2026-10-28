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
fn run_all_passing_fastq_assertions() {
    let output = exec(&["run", "tests/data/fastq_assertions.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!(
        "run_fastq_passing_stdout",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn assert_valid_plain_passes() {
    let output = exec(&["assert", "tests/data/sample.fastq fastq.valid eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_valid_gzip_passes() {
    let output = exec(&["assert", "tests/data/sample.fastq.gz fastq.valid eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn assert_truncated_fails_without_error() {
    let output = exec(&["assert", "tests/data/truncated.fastq fastq.valid eq true"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FAIL."), "stdout: {stdout}");
    assert!(
        stdout.contains("got false (record 1 (line 5): unexpected end of file)"),
        "stdout: {stdout}"
    );
}

#[test]
fn assert_truncated_gzip_fails_without_error() {
    let output = exec(&[
        "assert",
        "tests/data/truncated.fastq.gz fastq.valid eq true",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn assert_errors_on_missing_file() {
    let output = exec(&["assert", "tests/data/missing.fastq fastq.valid eq true"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn assert_errors_on_unsupported_compression() {
    let output = exec(&["assert", "tests/data/plain.txt.bz2 fastq.valid eq true"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR."), "stderr: {stderr}");
    assert!(
        stderr.contains("bzip2 compression is not supported"),
        "stderr: {stderr}"
    );
}

#[test]
fn guard_on_invalid_file_skips() {
    let output = exec(&[
        "assert",
        "tests/data/mismatch.fastq file.size gt 0B if tests/data/mismatch.fastq fastq.valid eq true",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SKIP."), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "got false (record 1 (line 5): quality length 4 does not match sequence length 8)"
        ),
        "stdout: {stdout}"
    );
}
