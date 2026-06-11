//! Command-line interface for Tauri release builds. This file should describe inputs, not perform builds.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tauri-release")]
#[command(bin_name = "tauri-release")]
#[command(
    author,
    version,
    about = "Release builder for Tauri desktop and mobile artifacts",
    after_help = "Read the full README: https://github.com/verzly/tauri-release"
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
#[command(after_help = "Read the full README: https://github.com/verzly/tauri-release")]
pub struct InitArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/tauri-release")]
pub struct CommonArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/tauri-release")]
pub struct BuildArgs {
    #[arg(short, long, default_value = "tauri-release.toml")]
    pub config: PathBuf,

    /// Build only this platform key.
    #[arg(long)]
    pub platform: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_build_platform_options() {
        let cli = Cli::parse_from([
            "tauri-release",
            "build",
            "--config",
            "tauri-release.toml",
            "--platform",
            "android",
            "--dry-run",
        ]);

        let Commands::Build(args) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(args.config, PathBuf::from("tauri-release.toml"));
        assert_eq!(args.platform.as_deref(), Some("android"));
        assert!(args.dry_run);
    }
}
