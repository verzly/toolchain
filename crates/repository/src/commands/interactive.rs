//! Shared interactive prompt helpers for command palette flows.

use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn prompt(label: &str) -> Result<String> {
    if label.ends_with('>') {
        print!("{label} ");
    } else {
        print!("{label}: ");
    }
    io::stdout().flush().context("failed to flush stdout")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .context("failed to read stdin")?;
    Ok(value.trim().to_string())
}

pub fn prompt_optional_path(label: &str) -> Result<Option<PathBuf>> {
    let value = prompt(&format!("{label} [use detected]"))?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

pub fn prompt_optional_path_with_default(label: &str, default: &str) -> Result<Option<PathBuf>> {
    let value = prompt(&format!("{label} [{default}]"))?;
    if value.trim().is_empty() {
        Ok(Some(PathBuf::from(default)))
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

pub fn prompt_default(label: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        prompt(label)
    } else {
        let value = prompt(&format!("{label} [{default}]"))?;
        if value.trim().is_empty() {
            Ok(default.to_string())
        } else {
            Ok(value)
        }
    }
}

pub fn confirm(label: &str) -> Result<bool> {
    let value = prompt(&format!("{label} [y/N]"))?;
    Ok(matches!(value.as_str(), "y" | "Y" | "yes" | "YES"))
}

pub fn pause() -> Result<()> {
    let _ = prompt("Press Enter to continue")?;
    Ok(())
}

pub fn wait_for_enter() -> Result<()> {
    println!();
    let _ = prompt("Press Enter to return to repository")?;
    Ok(())
}
