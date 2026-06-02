//! Application entry point. Keep command dispatch here and move workflow logic into command modules.

mod cli;
mod commands;
mod config;
mod domain;
mod git;
mod github;
mod output;
mod version_files;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Prepare(args) => commands::prepare::run(args),
        Commands::Finalize(args) => commands::finalize::run(args),
        Commands::Publish(args) => commands::publish::run(args),
        Commands::Abort(args) => commands::abort::run(args),
    }
}
