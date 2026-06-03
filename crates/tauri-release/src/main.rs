//! Application entry point. Platform-specific build details live outside the dispatch layer.

mod artifacts;
mod checksums;
mod cli;
mod commands;
mod config;
mod container;
mod manifest;
mod process;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Build(args) => commands::build::run(args),
        Commands::Clean(args) => commands::clean::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
