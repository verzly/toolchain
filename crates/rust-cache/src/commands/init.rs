//! Config bootstrap command. It writes cache settings into `datarose.toml` and native Cargo config.

use crate::cargo_config;
use crate::cli::InitArgs;
use crate::config;
use crate::env_plan::EnvPlan;
use anyhow::Result;

pub fn run(args: InitArgs) -> Result<()> {
    let config = config::ensure_config(&args.config, args.force)?;
    let cargo_config = cargo_config::write_workspace_config(&config, args.force)?;
    let plan = EnvPlan::build(&config)?;
    plan.ensure_runtime_files()?;

    println!("configured {}", args.config.display());
    println!("configured {}", cargo_config.display());
    if let Some(path) = &plan.gradle_init_script {
        println!("configured {}", path.display());
    }
    Ok(())
}
