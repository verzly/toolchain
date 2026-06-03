//! Config bootstrap command. Generated config should document the intended platform boundary.

use anyhow::Result;
use crate::cli::InitArgs;
use crate::config;

pub fn run(args: InitArgs) -> Result<()> {
    config::write_default_config(&args.config, args.force)?;
    println!("created {}", args.config.display());
    Ok(())
}
