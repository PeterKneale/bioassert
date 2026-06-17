mod cli;

use bioassert::parser::Assertion;
use clap::Parser;
use cli::{Cli, Commands};
use std::error::Error;
use std::fs;
use bioassert::executor::execute;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let all_passed = match cli.command {
        Commands::Assert { assertion } => {
            let assertion = bioassert::parser::parse_raw_assertion(&assertion)?;
            run_one(assertion)
        }

        Commands::Run { file } => {
            println!("Running assertions in {}", file.as_path().display());
            let contents = fs::read_to_string(file)?;
            match bioassert::parser::parse_file(&contents) {
                Ok(assertions) => assertions.into_iter().fold(true, |acc, a| run_one(a) && acc),
                Err(err) => {
                    eprintln!("💥{}", err);
                    return Err(err);
                }
            }
        }
    };

    if !all_passed {
        std::process::exit(1);
    }

    Ok(())
}

fn run_one(assertion: Assertion) -> bool {
    execute(assertion).unwrap_or_else(|error| { 
        eprintln!("ERROR. {}", error);
        false
    })
}
