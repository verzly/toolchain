//! Config bootstrap command. The generated config shows the defaults instead of hiding them in code.

use anyhow::Result;
use crate::cli::InitArgs;
use crate::config;

pub fn run(args: InitArgs) -> Result<()> {
    config::write_default_config(&args.config, args.force)?;
    println!("created {}", args.config.display());
    Ok(())
}
