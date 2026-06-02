//! Prints detected paths and environment decisions to help contributors understand why a cache path was chosen.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config;
use crate::env_plan::EnvPlan;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    println!("workspace: {}", plan.workspace_root.display());
    println!("package:   {}", plan.package);
    println!("cache:     {}", plan.cache_root.display());
    for (key, value) in &plan.values {
        println!("{key}: {value}");
    }
    Ok(())
}
