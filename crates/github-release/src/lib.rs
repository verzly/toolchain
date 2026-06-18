//! Reusable entry point for the `github-release` command.
//!
//! The standalone binary and the unified `verzly` binary both dispatch through this module so the
//! command contract stays in one place.

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
        Commands::Prepare(args) => commands::prepare::run(args),
        Commands::Finalize(args) => commands::finalize::run(args),
        Commands::FinalizeBatch(args) => commands::finalize::run_batch(args),
        Commands::Publish(args) => commands::publish::run(args),
        Commands::Delete(args) => commands::delete::run(args),
        Commands::FloatingTags(args) => commands::floating_tags::run(args),
        Commands::Abort(args) => commands::abort::run(args),
    }
}
