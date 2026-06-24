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
fn bgzf_file_is_detected_as_bgzf_not_gzip() {
    // sample.bam is a real bgzf (block-gzip) file: the heart of issue #6 is telling it
    // apart from plain gzip, since samtools and tabix require the bgzf variant.
    let output = exec(&["assert", "tests/data/sample.bam file.compression eq bgzf"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASS."),
        "expected PASS. in stdout: {stdout}"
    );
    assert!(
        stdout.contains("got bgzf"),
        "expected 'got bgzf' in stdout: {stdout}"
    );
}

#[test]
fn bgzf_file_is_not_plain_gzip() {
    let output = exec(&["assert", "tests/data/sample.bam file.compression ne gzip"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn plain_gzip_file_is_detected_as_gzip() {
    let output = exec(&["assert", "tests/data/plain.txt.gz file.compression eq gzip"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn plain_gzip_file_is_not_bgzf() {
    let output = exec(&["assert", "tests/data/plain.txt.gz file.compression eq bgzf"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("FAIL."),
        "expected FAIL. in stdout: {stdout}"
    );
    assert!(
        stdout.contains("got gzip"),
        "expected 'got gzip' in stdout: {stdout}"
    );
}

#[test]
fn uncompressed_file_is_none() {
    let output = exec(&["assert", "tests/data/example.tsv file.compression eq none"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn compression_label_matches_regex() {
    let output = exec(&[
        "assert",
        "tests/data/sample.bam file.compression matches '^b'",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn compressed_is_true_for_a_compressed_file() {
    let output = exec(&["assert", "tests/data/sample.bam file.compressed eq true"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn compressed_is_false_for_an_uncompressed_file() {
    let output = exec(&["assert", "tests/data/example.tsv file.compressed eq false"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn compression_guards_a_downstream_check() {
    // the canonical use: only run the BAM header check when the file really is bgzf
    let output = exec(&[
        "assert",
        "tests/data/sample.bam bam.header.rg.count gt 0 if tests/data/sample.bam file.compression eq bgzf",
    ]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn missing_file_errors() {
    let output = exec(&[
        "assert",
        "tests/data/nonexistent.gz file.compression eq gzip",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}

#[test]
fn numeric_comparator_on_label_errors() {
    let output = exec(&["assert", "tests/data/plain.txt.gz file.compression gt gzip"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ERROR."),
        "expected ERROR. in stderr: {stderr}"
    );
    assert!(
        stderr.contains("unsupported comparator"),
        "expected 'unsupported comparator' in stderr: {stderr}"
    );
}
