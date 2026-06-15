//! Prints shell exports for CI or scripts that want to apply the cache plan themselves.

use crate::cli::CommonArgs;
use crate::config;
use crate::env_plan::EnvPlan;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    plan.ensure_runtime_files()?;
    plan.print_exports();
    Ok(())
}
