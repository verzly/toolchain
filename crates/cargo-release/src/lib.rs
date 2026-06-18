//! Reusable entry point for the `cargo-release` command.
//!
//! The standalone binary and the unified `verzly` binary both dispatch through this module so the
//! command contract stays in one place.

mod artifacts;
mod checksums;
mod cli;
mod commands;
mod config;
mod container;
mod manifest;
mod process;

use crate::cli::{Cli, Commands};
use anyhow::Result;
use clap::Parser;
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
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Build(args) => commands::build::run(args),
        Commands::Clean(args) => commands::clean::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
