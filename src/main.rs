use clap::Parser;
use bioassert_engine::{executor, Assertion, AssertionReport, Outcome};

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
        fatal(&format!("could not write assertion report {}: {}", report_file.display(), e), color, icons);
    }

    std::process::exit(exit_code(&report));
}

/// Parses the assertion(s) named by the command. Returns a human-readable message on a
/// fatal failure (one that is not a per-assertion error): invalid syntax, or an
/// unreadable assertions file.
fn gather_assertions(command: &Commands) -> Result<Vec<Assertion>, String> {
    match command {
        Commands::Assert { assertion } => bioassert_engine::parser::parse_assertion(assertion)
            .map(|a| vec![a])
            .map_err(|e| e.to_string()),
        Commands::Run { file } => {
            let contents = std::fs::read_to_string(file).map_err(|e| format!("{}: {}", file.display(), e))?;
            bioassert_engine::parser::parse_file(&contents).map_err(|e| e.to_string())
        }
    }
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
    eprintln!("{}", report::format_outcome(Outcome::Error, message, color, icons));
    std::process::exit(2);
}
