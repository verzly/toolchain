//! Reusable entry point for the `ios-signing` command.
//!
//! The standalone binary and the unified `verzly` binary both dispatch through this module so the
//! command contract stays in one place.

mod cli;
mod commands;
mod ios;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use std::ffi::OsString;

pub fn run_from<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run(Cli::parse_from(args))
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Base64(args) => commands::base64::run(args),
        Commands::PrintSecrets(args) => commands::print_secrets::run(args),
        Commands::WriteGithubEnv(args) => commands::write_github_env::run(args),
        Commands::CheckEnv(args) => commands::check_env::run(args),
        Commands::Doctor => commands::doctor::run(),
    }
}
