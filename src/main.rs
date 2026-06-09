//! BioAssert CLI binary.
//!
//! Spec: `docs/spec.md` → "CLI Design", "Logging, Exit Codes, and Pipeline Behavior".
//!
//! Flow: parse args → open log file → read assertion files → `run_assertions` → print summary
//! to stdout/stderr, write the log, and exit with the report's exit code.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use bioassert::cli::Cli;
use bioassert::model::{Report, Status};
use bioassert::{Options, exit, run_assertions_with};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            // Usage / configuration / IO error → exit code 2 (spec).
            eprintln!("error: {err:#}");
            ExitCode::from(exit::USAGE_ERROR as u8)
        }
    }
}

fn run(cli: &Cli) -> Result<i32> {
    let mut log = open_log(&cli.log_file)
        .with_context(|| format!("failed to open log file {}", cli.log_file.display()))?;
    log_line(&mut log, "INFO", "starting bioassert");

    if cli.assertions.is_empty() {
        anyhow::bail!("no assertion files provided (use --assertions <file>)");
    }

    let source = read_assertion_files(&cli.assertions, &mut log)?;
    let inputs: HashMap<String, PathBuf> = cli.inputs.iter().cloned().collect();
    log_line(
        &mut log,
        "INFO",
        &format!("bound {} named input(s)", inputs.len()),
    );

    let options = Options {
        continue_on_failure: cli.continue_on_failure,
    };
    let report = run_assertions_with(&source, inputs, &options)?;

    print_report(cli, &report);
    write_report_to_log(&mut log, &report);

    Ok(report.exit_code())
}

/// Read and concatenate all assertion files into a single source string.
fn read_assertion_files(paths: &[PathBuf], log: &mut File) -> Result<String> {
    let mut source = String::new();
    for path in paths {
        log_line(
            log,
            "INFO",
            &format!("loading assertions {}", path.display()),
        );
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read assertion file {}", path.display()))?;
        source.push_str(&content);
        source.push('\n');
    }
    Ok(source)
}

/// Print the human-readable report to stdout (and a final summary).
fn print_report(cli: &Cli, report: &Report) {
    if !cli.quiet {
        for result in &report.results {
            let detail = if result.message.is_empty() {
                format!("{} {}", result.subject_label(), result.metric)
            } else {
                format!("{} {}", result.subject_label(), result.message)
            };
            println!("[{}] {detail}", result.status);
        }
    }

    if report.results.is_empty() {
        println!("No assertions to evaluate.");
        return;
    }

    println!("{} passed, {} failed", report.passed(), report.failed());
}

/// Append the report outcome to the log file.
fn write_report_to_log(log: &mut File, report: &Report) {
    for result in &report.results {
        let level = match result.status {
            Status::Pass => "INFO",
            Status::Fail => "ERROR",
        };
        log_line(
            log,
            level,
            &format!("{} {}", result.status, result.subject_label()),
        );
    }
    log_line(
        log,
        "INFO",
        &format!(
            "done: {} passed, {} failed",
            report.passed(),
            report.failed()
        ),
    );
}

/// Open (truncating) the log file for writing. The log file is always written (spec).
fn open_log(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    Ok(file)
}

/// Write a single log line. Phase 6 will add ISO-8601 timestamps and structured logging.
fn log_line(log: &mut File, level: &str, message: &str) {
    let _ = writeln!(log, "{level} {message}");
}
