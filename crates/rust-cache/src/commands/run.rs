//! Runs a user command with the planned cache environment. The tool should not interpret the command itself.

use crate::cli::RunArgs;
use crate::config;
use crate::env_plan::EnvPlan;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub fn run(args: RunArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = EnvPlan::build(&config)?;
    plan.ensure_runtime_files()?;

    let (program, rest) = args.command.split_first().context("missing command")?;
    let status = Command::new(program)
        .args(rest)
        .envs(&plan.values)
        .current_dir(&plan.workspace_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run {program}"))?;

    if !status.success() {
        anyhow::bail!("command failed: {}", args.command.join(" "));
    }

    Ok(())
}
