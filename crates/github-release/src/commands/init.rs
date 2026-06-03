//! Config bootstrap command. The generated file should be readable enough to edit by hand.

use anyhow::Result;

use crate::cli::InitArgs;
use crate::config;

pub fn run(args: InitArgs) -> Result<()> {
    config::write_default_config(&args.config, args.force)?;
    println!("created {}", args.config.display());
    Ok(())
}
