use std::process::{Command, Output};

fn exec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bioassert"))
        .args(args)
        .output()
        .expect("failed to run bioassert")
}

#[test]
fn no_args_exits_2_and_prints_help() {
    let output = exec(&[]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"), "expected usage in stderr: {stderr}");
}

#[test]
fn help_flag_exits_0_and_prints_usage() {
    let output = exec(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "expected usage: {stdout}");
    assert!(stdout.contains("-h, --help"), "expected help flag listed: {stdout}");
    assert!(stdout.contains("-V, --version"), "expected version flag listed: {stdout}");
}

#[test]
fn short_help_flag_exits_0() {
    let output = exec(&["-h"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}

#[test]
fn version_flag_prints_version() {
    let output = exec(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("bioassert "), "expected 'bioassert <version>': {stdout}");
}

#[test]
fn short_version_flag_prints_version() {
    let output = exec(&["-V"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("bioassert "), "expected 'bioassert <version>': {stdout}");
}
