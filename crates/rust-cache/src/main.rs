//! Application entry point. Keep command dispatch separate from workspace detection and environment planning.

mod cargo_config;
mod cli;
mod commands;
mod config;
mod env_plan;
mod generated;
mod workspace;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Env(args) => commands::env::run(args),
        Commands::Run(args) => commands::run::run(args),
        Commands::Clean(args) => commands::clean::run(args),
        Commands::CleanGenerated(args) => commands::clean_generated::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
