use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about,
    arg_required_else_help = true,
    disable_version_flag = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    // The version flag is defined manually (rather than via the auto flag `#[command(version)]`
    // adds) so its short form is the lowercase `-v` people expect, not clap's default `-V`.
    // `disable_version_flag` suppresses the auto flag while `version` still sets the version string.
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version, help = "Print the version")]
    pub version: Option<bool>,

    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "Write the assertion report to FILE instead of the default location"
    )]
    pub report_file: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        visible_alias = "colour",
        value_enum,
        default_value_t = When::Auto,
        value_name = "WHEN",
        help = "When to use ANSI color in console output: auto (only on a terminal), always, or never"
    )]
    pub color: When,

    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = When::Auto,
        value_name = "WHEN",
        help = "When to prefix PASS/FAIL/ERROR console lines with status icons: auto (only on a terminal), always, or never"
    )]
    pub icons: When,
}

/// Controls a terminal-sensitive console feature: ANSI color and status icons each
/// take one. The log file is unaffected by either, since it is read by tools that do
/// not interpret escape codes or care about decoration.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum When {
    /// On only when the console stream is a terminal and `NO_COLOR` is unset.
    Auto,
    /// Always on, regardless of terminal or `NO_COLOR`.
    Always,
    /// Always off.
    Never,
}

impl When {
    /// Resolves the choice into whether the feature should be active.
    ///
    /// `is_terminal` is whether the console stream is a TTY, and `no_color` is
    /// whether the `NO_COLOR` environment variable is set to a non-empty value.
    /// Both are consulted only for `Auto`; `Always` and `Never` are absolute so
    /// an explicit flag always overrides the environment.
    pub fn resolve(self, is_terminal: bool, no_color: bool) -> bool {
        match self {
            When::Always => true,
            When::Never => false,
            When::Auto => is_terminal && !no_color,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    Assert {
        assertion: String,
    },
    Run {
        #[arg(
            default_value = "assertions.txt",
            help = "Path to the assertions file to evaluate"
        )]
        file: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::When;

    #[test]
    fn always_is_on_regardless_of_environment() {
        assert!(When::Always.resolve(false, true));
        assert!(When::Always.resolve(false, false));
        assert!(When::Always.resolve(true, true));
    }

    #[test]
    fn never_is_off_regardless_of_environment() {
        assert!(!When::Never.resolve(true, false));
        assert!(!When::Never.resolve(true, true));
        assert!(!When::Never.resolve(false, false));
    }

    #[test]
    fn auto_is_on_only_on_a_terminal_without_no_color() {
        assert!(When::Auto.resolve(true, false));
        assert!(!When::Auto.resolve(true, true));
        assert!(!When::Auto.resolve(false, false));
        assert!(!When::Auto.resolve(false, true));
    }
}
