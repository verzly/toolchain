//! Version file updates for TOML, JSON, and plain text files. Commands decide when to call this; this module decides how values are written.

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::fs;

use crate::config::{VersionFileConfig, VersionFileKind};
use crate::domain::{render_template, ReleasePlan};

pub fn update_all(files: &[VersionFileConfig], plan: &ReleasePlan, dry_run: bool) -> Result<()> {
    for file in files {
        if !file.path.exists() {
            if file.optional {
                continue;
            }
            anyhow::bail!(
                "configured version file does not exist: {}",
                file.path.display()
            );
        }

        let rendered = render_value(file, plan);
        println!("update {} -> {}", file.path.display(), rendered);

        if dry_run {
            continue;
        }

        match file.kind {
            VersionFileKind::Toml => update_toml(file, &rendered)?,
            VersionFileKind::Json => update_json(file, &rendered)?,
            VersionFileKind::Text => update_text(file, plan)?,
        }
    }

    Ok(())
}

pub fn render_value(file: &VersionFileConfig, plan: &ReleasePlan) -> String {
    render_template(&file.value, &plan.tag, &plan.version_text)
}

fn update_toml(file: &VersionFileConfig, value: &str) -> Result<()> {
    let raw = fs::read_to_string(&file.path)?;
    let mut data: toml::Value = toml::from_str(&raw)?;
    set_toml_value(&mut data, &file.key, value)?;
    fs::write(&file.path, toml::to_string_pretty(&data)?)?;
    Ok(())
}

fn update_json(file: &VersionFileConfig, value: &str) -> Result<()> {
    let raw = fs::read_to_string(&file.path)?;
    let mut data: JsonValue = serde_json::from_str(&raw)?;
    set_json_value(&mut data, &file.key, value)?;
    fs::write(
        &file.path,
        format!("{}\n", serde_json::to_string_pretty(&data)?),
    )?;
    Ok(())
}

fn update_text(file: &VersionFileConfig, plan: &ReleasePlan) -> Result<()> {
    let raw = fs::read_to_string(&file.path)?;
    let replace = render_template(&file.replace, &plan.tag, &plan.version_text);

    // `{current}` means "replace the current trimmed contents" for plain text
    // files that intentionally store only the current release value.
    let search = if file.search == "{current}" {
        raw.trim().to_string()
    } else {
        render_template(&file.search, &plan.tag, &plan.version_text)
    };

    if search.is_empty() {
        anyhow::bail!("text version file requires search: {}", file.path.display());
    }

    if !raw.contains(&search) {
        anyhow::bail!("text search value was not found in {}", file.path.display());
    }

    fs::write(&file.path, raw.replace(&search, &replace))?;
    Ok(())
}

fn set_toml_value(root: &mut toml::Value, dotted_key: &str, value: &str) -> Result<()> {
    let parts: Vec<_> = dotted_key.split('.').collect();
    let mut current = root;
    for key in &parts[..parts.len() - 1] {
        current = current
            .get_mut(*key)
            .with_context(|| format!("missing TOML key: {key}"))?;
    }
    let last = parts.last().context("empty TOML key")?;
    let slot = current
        .get_mut(*last)
        .with_context(|| format!("missing TOML key: {last}"))?;
    *slot = toml::Value::String(value.to_string());
    Ok(())
}

fn set_json_value(root: &mut JsonValue, dotted_key: &str, value: &str) -> Result<()> {
    let parts: Vec<_> = dotted_key.split('.').collect();
    let mut current = root;
    for key in &parts[..parts.len() - 1] {
        current = current
            .get_mut(*key)
            .with_context(|| format!("missing JSON key: {key}"))?;
    }
    let last = parts.last().context("empty JSON key")?;
    current[*last] = JsonValue::String(value.to_string());
    Ok(())
}
