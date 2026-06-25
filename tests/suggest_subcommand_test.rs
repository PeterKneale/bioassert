use insta::assert_snapshot;
use std::process::{Command, Output};

/// Runs `suggest <input> --output <output>` (optionally with `--force`) against the compiled
/// binary, with color disabled so the output is plain text.
fn exec_suggest(input: &str, output: &str, force: bool) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bioassert"));
    cmd.arg("--color=never")
        .arg("suggest")
        .arg(input)
        .arg("--output")
        .arg(output);
    if force {
        cmd.arg("--force");
    }
    cmd.output().expect("failed to run bioassert")
}

/// Suggests assertions for `input` into a fresh temp directory and returns the written body.
/// The body references only the (repo-relative) input path, so it is deterministic; the temp
/// output path appears only in the success message, which is not snapshotted.
fn suggested_body(input: &str) -> String {
    let dir = tempfile::tempdir().expect("create temp dir");
    let out = dir.path().join("out.assertions.txt");
    let output = exec_suggest(input, out.to_str().unwrap(), false);
    assert!(
        output.status.success(),
        "suggest failed for {input}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&out).expect("read suggested output")
}

#[test]
fn suggest_tsv_snapshot() {
    assert_snapshot!("suggest_tsv", suggested_body("tests/data/junctions.tsv"));
}

#[test]
fn suggest_bam_snapshot() {
    assert_snapshot!("suggest_bam", suggested_body("tests/data/sample.bam"));
}

#[test]
fn suggest_fasta_snapshot() {
    assert_snapshot!("suggest_fasta", suggested_body("tests/data/sample.fasta"));
}

#[test]
fn suggest_unknown_extension_snapshot() {
    assert_snapshot!(
        "suggest_unknown_extension",
        suggested_body("tests/data/empty_file.txt")
    );
}

#[test]
fn refuses_to_overwrite_without_force() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let out = dir.path().join("out.assertions.txt");
    let out = out.to_str().unwrap();

    let first = exec_suggest("tests/data/junctions.tsv", out, false);
    assert!(first.status.success(), "first suggest should succeed");

    let second = exec_suggest("tests/data/junctions.tsv", out, false);
    assert_eq!(
        second.status.code(),
        Some(2),
        "a second run without --force must fail"
    );
    assert!(String::from_utf8_lossy(&second.stderr).contains("ERROR."));

    let forced = exec_suggest("tests/data/junctions.tsv", out, true);
    assert!(forced.status.success(), "--force must overwrite");
}

#[test]
fn suggested_assertions_round_trip_through_run() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let out = dir.path().join("out.assertions.txt");
    let out = out.to_str().unwrap();

    let suggested = exec_suggest("tests/data/junctions.tsv", out, false);
    assert!(suggested.status.success());

    // Every suggested assertion must hold against the file it was suggested from.
    let run = Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .arg("--color=never")
        .arg("run")
        .arg(out)
        .output()
        .expect("failed to run bioassert");
    assert!(
        run.status.success(),
        "round-trip run exited {}\nstdout:\n{}\nstderr:\n{}",
        run.status,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
}
