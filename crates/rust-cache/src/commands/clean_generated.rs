//! Removes reproducible build output directories that Tauri, Gradle, and Cargo may leave inside the project tree.

use crate::cli::CleanGeneratedArgs;
use crate::config;
use crate::env_plan::EnvPlan;
use crate::generated;
use anyhow::Result;
use std::fs;

pub fn run(args: CleanGeneratedArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    let paths = generated::discover_generated_outputs(
        &plan.workspace_root,
        &plan.cache_root,
        &config.generated.paths,
    )?;

    if paths.is_empty() {
        println!("no generated build outputs found");
        return Ok(());
    }

    for path in paths {
        if args.dry_run {
            println!("would remove {}", path.display());
        } else {
            fs::remove_dir_all(&path)?;
            println!("removed {}", path.display());
        }
    }

    Ok(())
}
