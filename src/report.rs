use bioassert::engine::{AssertionReport, AssertionResult, Outcome};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Resolves the assertion report path.
///
/// An explicit `--report-file` always wins. Otherwise the `run` subcommand derives
/// `<assertions-file>.log`, and everything else falls back to `assertions.log`. This
/// is the file the rendered report is written to; the application log is separate.
pub fn resolve_report_file(explicit: Option<PathBuf>, run_file: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Some(file) = run_file {
        return PathBuf::from(format!("{}.log", file.to_string_lossy()));
    }
    PathBuf::from("assertions.log")
}

/// Writes the assertion report to `path` as plain text (no color, no icons).
pub fn write_report(path: &Path, report: &AssertionReport) -> io::Result<()> {
    fs::write(path, report.render())
}

/// Prints the report to the console: PASS/FAIL on stdout, ERROR on stderr, each line
/// decorated with the keyword color and status icon according to `color` / `icons`.
pub fn print_report(report: &AssertionReport, color: bool, icons: bool) {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    for result in report.results() {
        let line = format_result(result, color, icons);
        let _ = match result.outcome {
            Outcome::Error => writeln!(err, "{line}"),
            _ => writeln!(out, "{line}"),
        };
    }
}

/// Formats a single result for the console, e.g. `🟢  PASS. Expected ...`.
pub fn format_result(result: &AssertionResult, color: bool, icons: bool) -> String {
    format_outcome(result.outcome, &result.message, color, icons)
}

/// Formats an outcome + message for the console. `color` colors the keyword (green
/// PASS, red FAIL/ERROR); `icons` prefixes a status icon (🟢 / 🔴 / 🔥). The two are
/// independent, so either may be on while the other is off. Used for assertion
/// results and for fatal application errors (reported as ERROR lines).
pub fn format_outcome(outcome: Outcome, message: &str, color: bool, icons: bool) -> String {
    let (icon, ansi) = match outcome {
        Outcome::Pass => ("🟢", "\x1b[32m"),
        Outcome::Fail => ("🔴", "\x1b[31m"),
        Outcome::Error => ("🔥", "\x1b[31m"),
        // SKIP is de-emphasised: a neutral icon and dim text, so it reads as "not run".
        Outcome::Skip => ("⚪", "\x1b[2m"),
    };
    // Two spaces keep the emoji from crowding the keyword in most terminals.
    let prefix = if icons {
        format!("{icon}  ")
    } else {
        String::new()
    };
    let label = outcome.label();
    if color {
        format!("{prefix}{ansi}{label}\x1b[0m. {message}")
    } else {
        format!("{prefix}{label}. {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_when_color_and_icons_off() {
        assert_eq!(
            format_outcome(Outcome::Pass, "ok", false, false),
            "PASS. ok"
        );
    }

    #[test]
    fn icon_prefix_when_icons_on() {
        assert_eq!(
            format_outcome(Outcome::Fail, "no", false, true),
            "🔴  FAIL. no"
        );
        assert_eq!(
            format_outcome(Outcome::Error, "boom", false, true),
            "🔥  ERROR. boom"
        );
    }

    #[test]
    fn colors_only_the_keyword_when_color_on() {
        assert_eq!(
            format_outcome(Outcome::Pass, "ok", true, false),
            "\x1b[32mPASS\x1b[0m. ok"
        );
    }

    #[test]
    fn color_and_icons_are_independent() {
        assert_eq!(
            format_outcome(Outcome::Pass, "ok", true, true),
            "🟢  \x1b[32mPASS\x1b[0m. ok"
        );
    }

    #[test]
    fn skip_renders_a_neutral_icon_and_label() {
        assert_eq!(
            format_outcome(Outcome::Skip, "guarded out", false, false),
            "SKIP. guarded out"
        );
        assert_eq!(
            format_outcome(Outcome::Skip, "guarded out", false, true),
            "⚪  SKIP. guarded out"
        );
    }

    #[test]
    fn derived_run_path_appends_log() {
        let p = resolve_report_file(None, Some(Path::new("checks.txt")));
        assert_eq!(p, PathBuf::from("checks.txt.log"));
    }

    #[test]
    fn explicit_path_wins() {
        let p = resolve_report_file(
            Some(PathBuf::from("out.log")),
            Some(Path::new("checks.txt")),
        );
        assert_eq!(p, PathBuf::from("out.log"));
    }

    #[test]
    fn default_path_is_assertions_log() {
        assert_eq!(
            resolve_report_file(None, None),
            PathBuf::from("assertions.log")
        );
    }
}
