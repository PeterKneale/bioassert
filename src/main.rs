mod cli;

use bioassert::parser::Assertion;
use clap::Parser;
use cli::{Cli, Commands};
use std::fs;

enum Outcome {
    Pass,
    Fail,
    Error,
}

fn main() {
    let cli = Cli::parse();

    let outcomes: Vec<Outcome> = match cli.command {
        Commands::Assert { assertion } => {
            let assertion = match assertion.parse::<Assertion>() {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("ERROR. {}", e);
                    std::process::exit(2);
                }
            };
            vec![run_one(assertion)]
        }

        Commands::Run { file } => {
            println!("Running assertions in {}", file.as_path().display());
            let contents = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("ERROR. {}: {}", file.display(), e);
                    std::process::exit(2);
                }
            };
            match bioassert::parser::parse_file(&contents) {
                Ok(assertions) => assertions.into_iter().map(run_one).collect(),
                Err(e) => {
                    eprintln!("ERROR. {}", e);
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
    match bioassert::executor::execute(assertion) {
        Ok(true) => Outcome::Pass,
        Ok(false) => Outcome::Fail,
        Err(e) => {
            eprintln!("ERROR. {}", e);
            Outcome::Error
        }
    }
}
