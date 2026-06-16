//! Prints detected paths and environment decisions to help contributors understand why a cache path was chosen.

use crate::cli::CommonArgs;
use crate::config;
use crate::env_plan::EnvPlan;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    plan.ensure_runtime_files()?;
    println!("workspace: {}", plan.workspace_root.display());
    println!("package:   {}", plan.package);
    println!("cache:     {}", plan.cache_root.display());
    println!("target:    {}", plan.cargo_target_dir.display());
    if let Some(path) = &plan.gradle_init_script {
        println!("gradle init: {}", path.display());
    }
    if let Some(path) = &plan.gradle_build_root {
        println!("gradle builds: {}", path.display());
    }
    for (key, value) in &plan.values {
        println!("{key}: {value}");
    }
    Ok(())
}
