//! Unified Verzly Toolchain entry point.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

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
            validate_datarose_before_tool_run(&args)?;
            github_release::run_from(with_tool_name("github-release", args))
        }
        Commands::CargoRelease(args) => {
            validate_datarose_before_tool_run(&args)?;
            cargo_release::run_from(with_tool_name("cargo-release", args))
        }
        Commands::TauriRelease(args) => {
            validate_datarose_before_tool_run(&args)?;
            tauri_release::run_from(with_tool_name("tauri-release", args))
        }
        Commands::RustCache(args) => {
            validate_datarose_before_tool_run(&args)?;
            rust_cache::run_from(with_tool_name("rust-cache", args))
        }
        Commands::AndroidSigning(args) => {
            validate_datarose_before_tool_run(&args)?;
            android_signing::run_from(with_tool_name("android-signing", args))
        }
        Commands::IosSigning(args) => {
            validate_datarose_before_tool_run(&args)?;
            ios_signing::run_from(with_tool_name("ios-signing", args))
        }
        Commands::Repository(args) => {
            validate_datarose_before_tool_run(&args)?;
            repository::run_from(with_tool_name("repository", args))
        }
    }
}

fn with_tool_name(tool: &'static str, args: PassthroughArgs) -> Vec<OsString> {
    let mut forwarded = Vec::with_capacity(args.args.len() + 1);
    forwarded.push(OsString::from(tool));
    forwarded.extend(args.args);
    forwarded
}

fn validate_datarose_before_tool_run(args: &PassthroughArgs) -> Result<()> {
    let config_path = datarose_config_path_from_args(args)
        .with_context(|| "failed to resolve datarose.toml validation path")?;
    repository::validate_datarose_for_tool_run(&config_path)
}

fn datarose_config_path_from_args(args: &PassthroughArgs) -> Result<PathBuf> {
    let root = option_value(&args.args, "--root")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let root = absolutize(root)?;

    if let Some(config) = option_value(&args.args, "--config") {
        let path = PathBuf::from(config);
        if path.file_name().and_then(|name| name.to_str()) == Some("datarose.toml") {
            return Ok(if path.is_absolute() {
                path
            } else {
                root.join(path)
            });
        }
    }

    Ok(root.join("datarose.toml"))
}

fn option_value(args: &[OsString], name: &str) -> Option<OsString> {
    let mut index = 0;
    while index < args.len() {
        let Some(value) = args[index].to_str() else {
            index += 1;
            continue;
        };

        if value == name {
            return args.get(index + 1).cloned();
        }

        if let Some((key, inline_value)) = value.split_once('=') {
            if key == name {
                return Some(OsString::from(inline_value));
            }
        }

        index += 1;
    }

    None
}

fn absolutize(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
