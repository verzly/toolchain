//! Small process helpers shared by the CLI tools.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};

pub fn shell(command: &str, env: &BTreeMap<String, String>, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{command}");
        return Ok(());
    }

    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-lc", command]);
        cmd
    };

    let status = cmd
        .envs(env)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to run command: {command}"))?;

    if !status.success() {
        anyhow::bail!("command failed: {command}");
    }

    Ok(())
}

pub fn available(executable: &str) -> bool {
    Command::new(executable)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
