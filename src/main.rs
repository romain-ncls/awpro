mod cli;
mod commands;
mod device;
mod error;
mod output;
mod protocol;

use clap::Parser;
use cli::{Cli, Command};
use error::AppError;
use hidapi::HidApi;

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    if let Err(e) = run(cli) {
        output::report_error(&e, json);
        std::process::exit(e.exit_code());
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    let json = cli.json;
    // One HidApi and one open per invocation: HidApi::new() enumerates every
    // HID device on the system, and every subcommand wants the same handle.
    let api = HidApi::new().map_err(|e| AppError::Init(e.to_string()))?;
    let (device, transport) = device::open(&api)?;

    let result = match cli.command {
        Command::Anc(cmd) => commands::anc::run(&device, cmd, json),
        Command::Mic(cmd) => commands::mic::run(&device, cmd, json),
        Command::Sidetone(cmd) => commands::sidetone::run(&device, cmd, json),
        Command::Power(cmd) => commands::power::run(&device, cmd, json),
        Command::Battery => commands::battery::run(&device, json),
        Command::Get(cmd) => commands::get::run(&device, cmd, json),
        Command::Status => commands::status::run(&device, json),
        Command::Info => commands::info::run(&device, transport, json),
        Command::Watch => commands::watch::run(&device, json),
    };

    // A timeout on the dongle usually means the headset is simply not on the
    // link. Saying so beats "device did not respond", which reads like a
    // driver fault.
    result.map_err(|e| device::diagnose(&device, transport, e))
}
