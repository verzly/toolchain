//! Human-readable and CI output helpers. GitHub Actions output writing stays here instead of being duplicated by commands.

use anyhow::{Context, Result};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use crate::domain::ReleasePlan;

pub fn print_plan(plan: &ReleasePlan) {
    println!("version:        {}", plan.version_text);
    println!("tag:            {}", plan.tag);
    println!("release name:   {}", plan.release_name);
    println!("target branch:  {}", plan.target_branch);
    println!("release branch: {}", plan.release_branch);
    println!("prerelease:     {}", plan.prerelease);
}

pub fn write_github_outputs(plan: &ReleasePlan) -> Result<()> {
    let Ok(path) = env::var("GITHUB_OUTPUT") else {
        return Ok(());
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open GITHUB_OUTPUT at {path}"))?;

    writeln!(file, "version={}", plan.version_text)?;
    writeln!(file, "tag={}", plan.tag)?;
    writeln!(file, "release_name={}", plan.release_name)?;
    writeln!(file, "target_branch={}", plan.target_branch)?;
    writeln!(file, "release_branch={}", plan.release_branch)?;
    writeln!(file, "prerelease={}", plan.prerelease)?;

    Ok(())
}
