mod cli;
mod commands;
mod device;
mod error;
mod output;

use clap::Parser;
use cli::{Cli, Command};
use error::AppError;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    let json = cli.json;
    match cli.command {
        Command::Anc(cmd) => commands::anc::run(cmd, json),
        Command::Mic(cmd) => commands::mic::run(cmd, json),
        Command::Sidetone(cmd) => commands::sidetone::run(cmd, json),
        Command::Power(cmd) => commands::power::run(cmd, json),
        Command::Battery => commands::battery::run(json),
        Command::Get(cmd) => commands::get::run(cmd, json),
    }
}
