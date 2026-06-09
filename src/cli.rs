//! Command-line interface definition.
//!
//! Spec: `docs/spec.md` → "CLI Design". BioAssert is a single-purpose, subcommand-less tool;
//! the root command evaluates assertions over bound inputs.

use std::path::PathBuf;

use clap::Parser;

/// A named input binding (`name=path`) from `--input`.
pub type InputBinding = (String, PathBuf);

#[derive(Debug, Parser)]
#[command(
    name = "bioassert",
    version,
    about = "Assert bioinformatics file properties.",
    long_about = None
)]
pub struct Cli {
    /// Path to an assertion file (plain text or YAML). May be repeated.
    #[arg(short = 'a', long = "assertions", value_name = "FILE")]
    pub assertions: Vec<PathBuf>,

    /// Bind a named input to a file: `name=path` (e.g. `bam=sample.bam`). May be repeated.
    #[arg(short = 'i', long = "input", value_name = "NAME=PATH", value_parser = parse_input)]
    pub inputs: Vec<InputBinding>,

    /// Write execution logs to the given file (default: `bioassert.log`).
    #[arg(
        short = 'l',
        long = "log-file",
        value_name = "FILE",
        default_value = "bioassert.log"
    )]
    pub log_file: PathBuf,

    /// Continue after failures and report all of them (alias: `--report-all`).
    #[arg(short = 'c', long = "continue", alias = "report-all")]
    pub continue_on_failure: bool,

    /// Minimal logging.
    #[arg(short = 'q', long = "quiet", conflicts_with = "verbose")]
    pub quiet: bool,

    /// Verbose logging.
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Parse a `--input` value of the form `name=path`.
fn parse_input(raw: &str) -> Result<InputBinding, String> {
    match raw.split_once('=') {
        Some((name, path)) if !name.is_empty() && !path.is_empty() => {
            Ok((name.to_string(), PathBuf::from(path)))
        }
        _ => Err(format!("invalid --input '{raw}', expected NAME=PATH")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_input_binding() {
        let (name, path) = parse_input("bam=sample.bam").expect("valid binding");
        assert_eq!(name, "bam");
        assert_eq!(path, PathBuf::from("sample.bam"));
    }

    #[test]
    fn rejects_input_without_equals() {
        assert!(parse_input("bam").is_err());
    }

    #[test]
    fn rejects_input_with_empty_side() {
        assert!(parse_input("=sample.bam").is_err());
        assert!(parse_input("bam=").is_err());
    }

    #[test]
    fn parses_path_containing_equals_sign() {
        let (name, path) = parse_input("ref=dir/key=value.fa").expect("split once only");
        assert_eq!(name, "ref");
        assert_eq!(path, PathBuf::from("dir/key=value.fa"));
    }

    #[test]
    fn cli_verifies() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
