//! Application entry point. Keep command dispatch here and move workflow logic into command modules.

mod cli;
mod commands;
mod config;
mod domain;
mod git;
mod github;
mod output;
mod version_files;

use crate::cli::{Cli, Commands};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Prepare(args) => commands::prepare::run(args),
        Commands::Finalize(args) => commands::finalize::run(args),
        Commands::FinalizeBatch(args) => commands::finalize::run_batch(args),
        Commands::Publish(args) => commands::publish::run(args),
        Commands::Delete(args) => commands::delete::run(args),
        Commands::FloatingTags(args) => commands::floating_tags::run(args),
        Commands::Abort(args) => commands::abort::run(args),
    }
}
