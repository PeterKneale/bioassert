use bioassert::engine::{Assertion, AssertionReport, Outcome, executor};
use clap::Parser;
use std::path::{Path, PathBuf};

mod cli;
mod report;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // Console color/icons are resolved against stdout (where PASS/FAIL land) and the
    // NO_COLOR convention (set and non-empty). They are independent toggles that share
    // the same resolution logic.
    let is_terminal = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let color = cli.color.resolve(is_terminal, no_color);
    let icons = cli.icons.resolve(is_terminal, no_color);

    // `suggest` is the inverse of evaluation: it writes an assertions file rather than
    // reading one, so it is handled here, before the gather/evaluate/report pipeline.
    if let Commands::Suggest {
        file,
        output,
        force,
    } = &cli.command
    {
        run_suggest(file, output.clone(), *force, color, icons);
    }

    let run_file = match &cli.command {
        Commands::Run { file } => Some(file.clone()),
        _ => None,
    };
    let report_file = report::resolve_report_file(cli.report_file, run_file.as_deref());

    // Collect the assertions to evaluate, or fail fatally (bad syntax / unreadable file).
    let assertions = match gather_assertions(&cli.command) {
        Ok(assertions) => assertions,
        Err(message) => fatal(&message, color, icons),
    };

    // Evaluate into a report, then present it: console first, then the persisted file.
    let report = executor::execute_all(assertions);
    report::print_report(&report, color, icons);
    if let Err(e) = report::write_report(&report_file, &report) {
        fatal(
            &format!(
                "could not write assertion report {}: {}",
                report_file.display(),
                e
            ),
            color,
            icons,
        );
    }

    std::process::exit(exit_code(&report));
}

/// Parses the assertion(s) named by the command. Returns a human-readable message on a
/// fatal failure (one that is not a per-assertion error): invalid syntax, or an
/// unreadable assertions file.
fn gather_assertions(command: &Commands) -> Result<Vec<Assertion>, String> {
    match command {
        Commands::Assert { assertion } => bioassert::engine::parser::parse_assertion(assertion)
            .map(|a| vec![a])
            .map_err(|e| e.to_string()),
        Commands::Run { file } => {
            let contents =
                std::fs::read_to_string(file).map_err(|e| format!("{}: {}", file.display(), e))?;
            bioassert::engine::parser::parse_file(&contents).map_err(|e| e.to_string())
        }
        Commands::Suggest { .. } => unreachable!("suggest is handled before gather_assertions"),
    }
}

/// Suggests assertions for `file` and writes them to the resolved output path, then exits. The
/// output path is `--output` if given, else `<file>.assertions.txt`; an existing output is not
/// overwritten without `--force`. Provider warnings are printed to stderr as ERROR lines but do
/// not change the exit code: a partial suggestion is still useful.
fn run_suggest(file: &Path, output: Option<PathBuf>, force: bool, color: bool, icons: bool) -> ! {
    let output_path = report::resolve_output_file(file, output);
    if output_path.exists() && !force {
        fatal(
            &format!(
                "output file {} already exists; pass --force to overwrite",
                output_path.display()
            ),
            color,
            icons,
        );
    }

    let result = bioassert::suggest::suggest(file);
    if result.suggestions.is_empty() {
        fatal(
            &format!("no assertions could be suggested for {}", file.display()),
            color,
            icons,
        );
    }

    for warning in &result.warnings {
        eprintln!(
            "{}",
            report::format_outcome(Outcome::Error, warning, color, icons)
        );
    }

    if let Err(e) = std::fs::write(&output_path, &result.rendered) {
        fatal(
            &format!("could not write {}: {}", output_path.display(), e),
            color,
            icons,
        );
    }

    println!(
        "Wrote {} assertions to {}",
        result.suggestions.len(),
        output_path.display()
    );
    println!(
        "Review and tighten them, then run: bioassert run {}",
        output_path.display()
    );
    std::process::exit(0);
}

/// The worst outcome across the report determines the exit code: 2 if any assertion
/// errored, 1 if any failed, otherwise 0.
fn exit_code(report: &AssertionReport) -> i32 {
    if report.has_errors() {
        2
    } else if report.has_failures() {
        1
    } else {
        0
    }
}

/// Reports a fatal application error to stderr (as an ERROR line) and exits 2.
fn fatal(message: &str, color: bool, icons: bool) -> ! {
    eprintln!(
        "{}",
        report::format_outcome(Outcome::Error, message, color, icons)
    );
    std::process::exit(2);
}
