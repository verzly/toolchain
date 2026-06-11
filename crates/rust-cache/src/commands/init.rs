//! Config bootstrap command. It writes cache settings into `datarose.toml` and native Cargo config.

use crate::cargo_config;
use crate::cli::InitArgs;
use crate::config;
use anyhow::Result;

pub fn run(args: InitArgs) -> Result<()> {
    let config = config::ensure_config(&args.config, args.force)?;
    let cargo_config = cargo_config::write_workspace_config(&config, args.force)?;

    println!("configured {}", args.config.display());
    println!("configured {}", cargo_config.display());
    Ok(())
}
