//! Reusable entry point for the `rust-cache` command.
//!
//! The standalone binary and the unified `verzly` binary both dispatch through this module so the
//! command contract stays in one place.

mod cargo_config;
mod cli;
mod commands;
mod config;
mod env_plan;
mod generated;
mod gradle_cache;
mod workspace;

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
        Commands::Init(args) => commands::init::run(args),
        Commands::Env(args) => commands::env::run(args),
        Commands::Run(args) => commands::run::run(args),
        Commands::Clean(args) => commands::clean::run(args),
        Commands::CleanGenerated(args) => commands::clean_generated::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
