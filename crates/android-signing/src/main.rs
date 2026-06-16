//! Application entry point. Signing commands are dispatched here; keystore handling stays in dedicated modules.

mod android;
mod cli;
mod commands;
mod secrets;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate(args) => commands::generate::run(args),
        Commands::Base64(args) => commands::base64::run(args),
        Commands::Fingerprint(args) => commands::fingerprint::run(args),
        Commands::VerifyFingerprint(args) => commands::verify_fingerprint::run(args),
        Commands::PrintSecrets(args) => commands::print_secrets::run(args),
        Commands::WriteGithubEnv(args) => commands::write_github_env::run(args),
        Commands::CheckEnv(args) => commands::check_env::run(args),
        Commands::Doctor => commands::doctor::run(),
    }
}
