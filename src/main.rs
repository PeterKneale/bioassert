use clap::{Parser, Subcommand};
use std::path::PathBuf;
use bioassert_engine::Assertion;
use std::fmt;
use std::fs;

use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Formats an event as its message only (no timestamp/level/target), coloring the
/// line green for PASS, red for FAIL/ERROR. Color is applied only when the layer's
/// writer has ANSI enabled, so it follows the layer's `with_ansi` setting.
struct ConsoleMessageFormat;

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }
}

impl<S, N> FormatEvent<S, N> for ConsoleMessageFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message.unwrap_or_default();

        // Color only the leading keyword (PASS/FAIL/ERROR), leaving the rest plain.
        let keyword = if writer.has_ansi_escapes() {
            [("PASS", "\x1b[32m"), ("FAIL", "\x1b[31m"), ("ERROR", "\x1b[31m")]
                .into_iter()
                .find(|(word, _)| message.starts_with(word))
        } else {
            None
        };

        match keyword {
            Some((word, color)) => {
                writeln!(writer, "{}{}\x1b[0m{}", color, word, &message[word.len()..])
            }
            None => writeln!(writer, "{}", message),
        }
    }
}

#[derive(Parser)]
#[command(version, about, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, help = "Enable debug logging to stderr")]
    pub verbose: bool,

    #[arg(long, value_name = "FILE", help = "Write logs to FILE instead of the default location")]
    pub log_file: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = true,
        action = clap::ArgAction::Set,
        help = "Enable ANSI color in console log output (default: true, use --color-console-log=false to disable)"
    )]
    pub color_console_log: bool,

    #[arg(
        long,
        default_value_t = false,
        action = clap::ArgAction::Set,
        help = "Enable ANSI color in file log output (default: false, use --color-file-log=true to enable)"
    )]
    pub color_file_log: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Assert {
        assertion: String,
    },
    Run {
        file: PathBuf,
    },
}

enum Outcome {
    Pass,
    Fail,
    Error,
}

fn resolve_log_file(explicit: Option<PathBuf>, command: &Commands) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Commands::Run { file } = command {
        return PathBuf::from(format!("{}.log", file.to_string_lossy()));
    }
    PathBuf::from("assertions.log")
}

fn init_logging(verbose: bool, log_file: PathBuf, color_console: bool, color_file: bool) {
    use tracing::Level;
    use tracing_subscriber::{filter::{filter_fn, LevelFilter}, fmt, prelude::*};

    let file = match fs::File::create(&log_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR. Could not create log file {}: {}", log_file.display(), e);
            std::process::exit(2);
        }
    };

    // INFO events (PASS/FAIL/Running) → stdout, message only, content-colored
    let stdout_layer = fmt::layer()
        .with_ansi(color_console)
        .with_writer(std::io::stdout)
        .event_format(ConsoleMessageFormat)
        .with_filter(filter_fn(|m| *m.level() == Level::INFO));

    // ERROR events → stderr, message only, content-colored (always)
    let stderr_error_layer = fmt::layer()
        .with_ansi(color_console)
        .with_writer(std::io::stderr)
        .event_format(ConsoleMessageFormat)
        .with_filter(filter_fn(|m| *m.level() == Level::ERROR));

    // DEBUG/TRACE → stderr with metadata (only when --verbose, avoids duplicating INFO/ERROR)
    let stderr_verbose_layer = verbose.then(|| {
        fmt::layer()
            .with_ansi(color_console)
            .with_writer(std::io::stderr)
            .with_filter(filter_fn(|m| {
                *m.level() == Level::DEBUG || *m.level() == Level::TRACE
            }))
    });

    // DEBUG+ → file with metadata (always)
    let file_layer = fmt::layer()
        .with_ansi(color_file)
        .with_writer(std::sync::Mutex::new(file))
        .with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(stderr_error_layer)
        .with(stderr_verbose_layer)
        .with(file_layer)
        .init();
}

fn main() {
    let cli = Cli::parse();

    let log_file = resolve_log_file(cli.log_file, &cli.command);
    init_logging(cli.verbose, log_file, cli.color_console_log, cli.color_file_log);

    let outcomes: Vec<Outcome> = match cli.command {
        Commands::Assert { assertion } => {
            tracing::debug!("parsing assertion: {}", assertion);
            let assertion = match bioassert_engine::parser::parse_assertion(&assertion) {
                Ok(a) => a,
                Err(e) => {
                    tracing::error!("ERROR. {}", e);
                    std::process::exit(2);
                }
            };
            vec![run_one(assertion)]
        }

        Commands::Run { file } => {
            tracing::debug!("reading assertions from: {}", file.display());
            tracing::info!("Running assertions in {}", file.as_path().display());
            let contents = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("ERROR. {}: {}", file.display(), e);
                    std::process::exit(2);
                }
            };
            match bioassert_engine::parser::parse_file(&contents) {
                Ok(assertions) => {
                    tracing::debug!("parsed {} assertion(s)", assertions.len());
                    assertions.into_iter().map(run_one).collect()
                }
                Err(e) => {
                    tracing::error!("ERROR. {}", e);
                    std::process::exit(2);
                }
            }
        }
    };

    if outcomes.iter().any(|o| matches!(o, Outcome::Error)) {
        std::process::exit(2);
    }
    if outcomes.iter().any(|o| matches!(o, Outcome::Fail)) {
        std::process::exit(1);
    }
}

fn run_one(assertion: Assertion) -> Outcome {
    match bioassert_engine::executor::execute(assertion) {
        Ok(true) => Outcome::Pass,
        Ok(false) => Outcome::Fail,
        Err(e) => {
            tracing::error!("ERROR. {}", e);
            Outcome::Error
        }
    }
}
