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
    let output = exec(&["run", "tests/data/resource_types.txt"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_snapshot!("run_resource_stdout", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn text_length_passes() {
    let output = exec(&["assert", "'abc' text.length gt 2"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn text_length_fails() {
    let output = exec(&["assert", "'abc' text.length gt 5"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("FAIL."));
}

#[test]
fn text_value_matches_regex() {
    let output = exec(&["assert", "'NC_000001.11' text.value matches '^NC_'"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn quoted_locator_with_space_resolves() {
    // The central strip_quotes unwraps the locator, and the quoted-first grammar keeps the
    // space-containing path whole, so the file metric opens it.
    let output = exec(&["assert", "'tests/data/with space.txt' file.size gt 0B"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS."));
}

#[test]
fn mismatched_metric_on_url_locator_errors() {
    // file.size on a URL-looking locator selects the file executor, which fails to open it.
    let output = exec(&["assert", "http://example.com file.size gt 0B"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR."));
}
