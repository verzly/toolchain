//! Unified Verzly Toolchain entry point.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;

#[derive(Parser, Debug)]
#[command(name = "verzly")]
#[command(bin_name = "verzly")]
#[command(author, version)]
#[command(about = "Unified entrypoint for the Verzly release and repository toolchain")]
#[command(
    after_help = "Run `verzly <tool> --help` for tool-specific help, for example `verzly github-release --help`."
)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// GitHub release branch, version, tag, asset, and publish orchestration.
    #[command(
        name = "github-release",
        visible_alias = "gh-release",
        trailing_var_arg = true
    )]
    GithubRelease(PassthroughArgs),

    /// Rust executable release artifact builder.
    #[command(name = "cargo-release", trailing_var_arg = true)]
    CargoRelease(PassthroughArgs),

    /// Tauri desktop and mobile release artifact builder.
    #[command(
        name = "tauri-release",
        visible_alias = "tauri",
        trailing_var_arg = true
    )]
    TauriRelease(PassthroughArgs),

    /// Rust, Gradle, JavaScript, and generated output cache routing helper.
    #[command(name = "rust-cache", visible_alias = "cache", trailing_var_arg = true)]
    RustCache(PassthroughArgs),

    /// Android release signing keystore helper.
    #[command(
        name = "android-signing",
        visible_alias = "android",
        trailing_var_arg = true
    )]
    AndroidSigning(PassthroughArgs),

    /// iOS signing secret and environment helper.
    #[command(name = "ios-signing", visible_alias = "ios", trailing_var_arg = true)]
    IosSigning(PassthroughArgs),

    /// Repository standards manager for hk, mise, workflows, and quality configs.
    #[command(name = "repository", visible_alias = "repo", trailing_var_arg = true)]
    Repository(PassthroughArgs),
}

#[derive(Args, Debug)]
struct PassthroughArgs {
    /// Arguments passed to the selected tool.
    #[arg(num_args = 0.., allow_hyphen_values = true, trailing_var_arg = true)]
    args: Vec<OsString>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GithubRelease(args) => {
            github_release::run_from(with_tool_name("github-release", args))
        }
        Commands::CargoRelease(args) => {
            cargo_release::run_from(with_tool_name("cargo-release", args))
        }
        Commands::TauriRelease(args) => {
            tauri_release::run_from(with_tool_name("tauri-release", args))
        }
        Commands::RustCache(args) => rust_cache::run_from(with_tool_name("rust-cache", args)),
        Commands::AndroidSigning(args) => {
            android_signing::run_from(with_tool_name("android-signing", args))
        }
        Commands::IosSigning(args) => ios_signing::run_from(with_tool_name("ios-signing", args)),
        Commands::Repository(args) => repository::run_from(with_tool_name("repository", args)),
    }
}

fn with_tool_name(tool: &'static str, args: PassthroughArgs) -> Vec<OsString> {
    let mut forwarded = Vec::with_capacity(args.args.len() + 1);
    forwarded.push(OsString::from(tool));
    forwarded.extend(args.args);
    forwarded
}
