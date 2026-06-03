//! Command-line interface for Tauri release builds. This file should describe inputs, not perform builds.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tauri-release")]
#[command(bin_name = "tauri-release")]
#[command(
    author,
    version,
    about = "Release builder for Tauri desktop and mobile artifacts"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Write a starter tauri-release.toml file.
    Init(InitArgs),
    /// Print the configured build plan.
    Plan(CommonArgs),
    /// Build release artifacts.
    Build(BuildArgs),
    /// Remove output and cache directories.
    Clean(CommonArgs),
    /// Check local tooling.
    Doctor(CommonArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CommonArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,

    /// Build only this platform key.
    #[arg(long)]
    pub platform: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}
