//! Prints shell exports for CI or scripts that want to apply the cache plan themselves.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config;
use crate::env_plan::EnvPlan;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    EnvPlan::build(&config)?.print_exports();
    Ok(())
}
