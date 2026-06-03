//! Config bootstrap command. The default config is a starting point, not hidden behavior.

use anyhow::Result;
use crate::cli::InitArgs;
use crate::config;

pub fn run(args: InitArgs) -> Result<()> {
    config::write_default_config(&args.config, args.force)?;
    println!("created {}", args.config.display());
    Ok(())
}
