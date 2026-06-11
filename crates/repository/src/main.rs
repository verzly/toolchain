//! Entry point for repository standards bootstrap commands.

mod cli;
mod commands;
mod project;
mod quality;
mod shell;
mod standards;
mod workflow;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Update(args) => commands::init::run_update(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Check(args) => commands::check::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
