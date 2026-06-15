//! Application entry point. iOS signing commands stay small because certificates and profiles are long-lived secrets.

mod cli;
mod commands;
mod ios;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Base64(args) => commands::base64::run(args),
        Commands::PrintSecrets(args) => commands::print_secrets::run(args),
        Commands::WriteGithubEnv(args) => commands::write_github_env::run(args),
        Commands::CheckEnv(args) => commands::check_env::run(args),
        Commands::Doctor => commands::doctor::run(),
    }
}
