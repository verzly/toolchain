//! Command-line interface for the builder. Keep this file focused on flags and subcommands.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cargo-release")]
#[command(bin_name = "cargo-release")]
#[command(
    author,
    version,
    about = "Container-aware release builder for Rust executable artifacts"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Write a starter cargo-release.toml file.
    Init(InitArgs),
    /// Print the configured build plan.
    Plan(CommonArgs),
    /// Build release artifacts.
    Build(BuildArgs),
    /// Remove generated release artifacts.
    Clean(CommonArgs),
    /// Check local tooling.
    Doctor(CommonArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(short, long, default_value = "cargo-release.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CommonArgs {
    #[arg(short, long, default_value = "cargo-release.toml")]
    pub config: PathBuf,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long, default_value = "cargo-release.toml")]
    pub config: PathBuf,

    /// Version used when naming release artifacts.
    #[arg(short, long)]
    pub version: Option<String>,

    /// Build only this target key.
    #[arg(long)]
    pub target: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_build_options() {
        let cli = Cli::parse_from([
            "cargo-release",
            "build",
            "--config",
            "release.toml",
            "--version",
            "1.2.3",
            "--target",
            "linux-x64",
            "--dry-run",
        ]);

        let Commands::Build(args) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(args.config, PathBuf::from("release.toml"));
        assert_eq!(args.version.as_deref(), Some("1.2.3"));
        assert_eq!(args.target.as_deref(), Some("linux-x64"));
        assert!(args.dry_run);
    }
}
