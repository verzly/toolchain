//! Reusable entry point for the `repository` command.
//!
//! The standalone binary and the unified `verzly` binary both dispatch through this module so the
//! command contract stays in one place.

mod cli;
mod commands;
mod output;
mod project;
mod quality;
mod release;
mod schema;
mod shell;
mod standards;
mod workflow;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, TuiArgs};
use std::ffi::OsString;
use std::path::Path;

pub fn run_from<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run(Cli::parse_from(args))
}

/// Validate an existing datarose.toml file with the same directive and embedded schema checks used
/// by `repository check`. Missing files are ignored so `verzly` can run outside a managed repo.
pub fn validate_datarose_for_tool_run(path: &Path) -> Result<()> {
    schema::validate_datarose_for_tool_run(path)
}

fn run(cli: Cli) -> Result<()> {
    let command = cli.command.unwrap_or_else(|| {
        Commands::Tui(TuiArgs {
            root: ".".into(),
            config: None,
        })
    });
    match command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Update(args) => commands::init::run_update(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Projects(args) => commands::projects::run(args),
        Commands::Check(args) => commands::check::run(args),
        Commands::Release(args) => commands::release::run(*args),
        Commands::Tui(args) => commands::tui::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
