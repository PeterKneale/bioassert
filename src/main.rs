mod files;
mod assertions;
mod cli;
mod executor;
mod parser;

use std::error::Error;
use clap::Parser;
use cli::{Cli, Commands};
use std::fs;
use crate::parser::Assertion;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Assert { assertion } => {
            let assertion = parser::parse_raw_assertion(&assertion)?;
            execute(assertion);
            Ok(())
        }

        Commands::Run { file } => {
            println!("Running assertions in {}",file.as_path().display());
            let contents = fs::read_to_string(file)?;
            match parser::parse_file(&contents){
                Ok(assertions) => {
                    for assertion in assertions {
                        execute(assertion);
                    }
                }
                Err(err) => {
                    eprintln!("Error parsing assertions. {}", err);
                    return Err(err.into());
                }
            }
            Ok(())
        }
    }
}

fn execute(assertion: Assertion) {
    match executor::execute(assertion) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("ERROR. {}", error);
        }
    }
}