//! Integration tests for the BioAssert CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("bioassert").expect("bioassert binary not found")
}

// --- --help / --version ---

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("BioAssert"));
}

#[test]
fn test_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bioassert"));
}

// --- validate-dna ---

#[test]
fn test_validate_dna_valid() {
    cmd()
        .args(["validate-dna", "ATGCN"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_validate_dna_invalid() {
    cmd()
        .args(["validate-dna", "ATGX"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_validate_dna_empty() {
    cmd()
        .args(["validate-dna", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// --- validate-rna ---

#[test]
fn test_validate_rna_valid() {
    cmd()
        .args(["validate-rna", "AUGCN"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_validate_rna_invalid() {
    cmd()
        .args(["validate-rna", "AUGCT"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// --- validate-protein ---

#[test]
fn test_validate_protein_valid() {
    cmd()
        .args(["validate-protein", "MSTV"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_validate_protein_invalid() {
    cmd()
        .args(["validate-protein", "MST2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// --- gc-content ---

#[test]
fn test_gc_content_default_range() {
    cmd()
        .args(["gc-content", "ATGC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("GC content"));
}

#[test]
fn test_gc_content_custom_range_pass() {
    cmd()
        .args(["gc-content", "ATGC", "--min", "0.4", "--max", "0.6"])
        .assert()
        .success();
}

#[test]
fn test_gc_content_custom_range_fail() {
    cmd()
        .args(["gc-content", "AAAA", "--min", "0.4", "--max", "0.6"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// --- check-length ---

#[test]
fn test_check_length_pass() {
    cmd()
        .args(["check-length", "ATGC", "--min", "1", "--max", "10"])
        .assert()
        .success();
}

#[test]
fn test_check_length_fail() {
    cmd()
        .args(["check-length", "ATGCATGCATGC", "--min", "1", "--max", "5"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// --- no subcommand ---

#[test]
fn test_no_args_shows_help() {
    cmd().assert().failure();
}
