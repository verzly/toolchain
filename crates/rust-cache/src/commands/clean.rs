//! Removes the configured cache root. This command must stay scoped to paths owned by `rust-cache`.

use anyhow::Result;
use std::fs;

use crate::cli::CommonArgs;
use crate::config;
use crate::env_plan::EnvPlan;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    if plan.cache_root.exists() {
        fs::remove_dir_all(&plan.cache_root)?;
        println!("removed {}", plan.cache_root.display());
    }
    Ok(())
}
