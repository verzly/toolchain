//! Version file updates for TOML, JSON, Cargo.lock package entries, and plain text files.
//! Commands decide when to call this; this module decides how values are written.

use crate::config::{VersionFileConfig, VersionFileKind, VersionValueType};
use crate::domain::{render_template, ReleasePlan};
use anyhow::{Context, Result};
use serde_json::{Number as JsonNumber, Value as JsonValue};
use std::fs;

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
            VersionFileKind::CargoLockPackage => update_cargo_lock_package(file, &rendered)?,
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
    set_toml_value(&mut data, &file.key, value, file.value_type)?;
    fs::write(&file.path, toml::to_string_pretty(&data)?)?;
    Ok(())
}

fn update_json(file: &VersionFileConfig, value: &str) -> Result<()> {
    let raw = fs::read_to_string(&file.path)?;
    let mut data: JsonValue = serde_json::from_str(&raw)?;
    set_json_value(&mut data, &file.key, value, file.value_type)?;
    fs::write(
        &file.path,
        format!("{}\n", serde_json::to_string_pretty(&data)?),
    )?;
    Ok(())
}

fn update_cargo_lock_package(file: &VersionFileConfig, value: &str) -> Result<()> {
    if file.package.trim().is_empty() {
        anyhow::bail!(
            "cargo-lock-package version file requires package: {}",
            file.path.display()
        );
    }

    let raw = fs::read_to_string(&file.path)?;
    let mut data: toml::Value = toml::from_str(&raw)?;
    let packages = data
        .get_mut("package")
        .and_then(toml::Value::as_array_mut)
        .with_context(|| {
            format!(
                "{} does not contain Cargo.lock package entries",
                file.path.display()
            )
        })?;
    let key = if file.key.trim().is_empty() {
        "version"
    } else {
        file.key.as_str()
    };

    for package in packages {
        let Some(table) = package.as_table_mut() else {
            continue;
        };
        let is_match = table
            .get("name")
            .and_then(toml::Value::as_str)
            .map(|name| name == file.package)
            .unwrap_or(false);
        if is_match {
            table.insert(key.to_string(), toml::Value::String(value.to_string()));
            fs::write(&file.path, toml::to_string_pretty(&data)?)?;
            return Ok(());
        }
    }

    anyhow::bail!(
        "Cargo.lock package `{}` was not found in {}",
        file.package,
        file.path.display()
    )
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

fn toml_value(value: &str, value_type: VersionValueType) -> Result<toml::Value> {
    match value_type {
        VersionValueType::String => Ok(toml::Value::String(value.to_string())),
        VersionValueType::Integer => {
            Ok(toml::Value::Integer(value.parse::<i64>().with_context(
                || format!("version value `{value}` is not a valid integer"),
            )?))
        }
    }
}

fn json_value(value: &str, value_type: VersionValueType) -> Result<JsonValue> {
    match value_type {
        VersionValueType::String => Ok(JsonValue::String(value.to_string())),
        VersionValueType::Integer => Ok(JsonValue::Number(JsonNumber::from(
            value
                .parse::<i64>()
                .with_context(|| format!("version value `{value}` is not a valid integer"))?,
        ))),
    }
}

fn set_toml_value(
    root: &mut toml::Value,
    dotted_key: &str,
    value: &str,
    value_type: VersionValueType,
) -> Result<()> {
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
    *slot = toml_value(value, value_type)?;
    Ok(())
}

fn set_json_value(
    root: &mut JsonValue,
    dotted_key: &str,
    value: &str,
    value_type: VersionValueType,
) -> Result<()> {
    let parts: Vec<_> = dotted_key.split('.').collect();
    let mut current = root;
    for key in &parts[..parts.len() - 1] {
        current = current
            .get_mut(*key)
            .with_context(|| format!("missing JSON key: {key}"))?;
    }
    let last = parts.last().context("empty JSON key")?;
    current[*last] = json_value(value, value_type)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, VersionFileConfig, VersionFileKind, VersionValueType};
    use crate::domain::build_plan;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("github-release-{name}-{suffix}"));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn plan() -> crate::domain::ReleasePlan {
        build_plan(&Config::default(), "1.2.3", None, None, None).expect("valid plan")
    }

    fn version_file(path: &Path, kind: VersionFileKind, key: &str) -> VersionFileConfig {
        VersionFileConfig {
            path: path.to_path_buf(),
            kind,
            key: key.to_string(),
            value: "{version}".to_string(),
            search: "{current}".to_string(),
            replace: "{tag}".to_string(),
            optional: false,
            ..VersionFileConfig::default()
        }
    }

    #[test]
    fn updates_toml_json_and_text_version_files() {
        let dir = temp_dir("version-files");
        let toml_path = dir.join("Cargo.toml");
        let json_path = dir.join("package.json");
        let text_path = dir.join("VERSION");

        std::fs::write(
            &toml_path,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write TOML");
        std::fs::write(&json_path, "{\"package\":{\"version\":\"0.1.0\"}}").expect("write JSON");
        std::fs::write(&text_path, "0.1.0\n").expect("write text");

        let files = [
            version_file(&toml_path, VersionFileKind::Toml, "package.version"),
            version_file(&json_path, VersionFileKind::Json, "package.version"),
            version_file(&text_path, VersionFileKind::Text, ""),
        ];

        update_all(&files, &plan(), false).expect("update files");

        assert!(std::fs::read_to_string(&toml_path)
            .expect("read TOML")
            .contains("version = \"1.2.3\""));
        assert!(std::fs::read_to_string(&json_path)
            .expect("read JSON")
            .contains("\"version\": \"1.2.3\""));
        assert_eq!(
            std::fs::read_to_string(&text_path).expect("read text"),
            "v1.2.3\n"
        );
    }

    #[test]
    fn updates_json_integer_values() {
        let dir = temp_dir("json-integer");
        let json_path = dir.join("tauri.conf.json");
        std::fs::write(
            &json_path,
            "{\"bundle\":{\"android\":{\"versionCode\":100}}}",
        )
        .expect("write JSON");

        let file = VersionFileConfig {
            path: json_path.clone(),
            kind: VersionFileKind::Json,
            key: "bundle.android.versionCode".to_string(),
            value: "{android_version_code}".to_string(),
            value_type: VersionValueType::Integer,
            ..VersionFileConfig::default()
        };

        update_all(&[file], &plan(), false).expect("update integer");

        assert!(std::fs::read_to_string(&json_path)
            .expect("read JSON")
            .contains("\"versionCode\": 10203"));
    }

    #[test]
    fn updates_cargo_lock_package_version() {
        let dir = temp_dir("cargo-lock");
        let lock_path = dir.join("Cargo.lock");
        std::fs::write(
            &lock_path,
            "[[package]]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"other\"\nversion = \"0.1.0\"\n",
        )
        .expect("write Cargo.lock");

        let file = VersionFileConfig {
            path: lock_path.clone(),
            kind: VersionFileKind::CargoLockPackage,
            key: "version".to_string(),
            package: "demo".to_string(),
            ..VersionFileConfig::default()
        };

        update_all(&[file], &plan(), false).expect("update Cargo.lock package");
        let updated = std::fs::read_to_string(&lock_path).expect("read Cargo.lock");

        assert!(updated.contains("name = \"demo\"\nversion = \"1.2.3\""));
        assert!(updated.contains("name = \"other\"\nversion = \"0.1.0\""));
    }

    #[test]
    fn skips_missing_optional_files() {
        let file = VersionFileConfig {
            path: temp_dir("optional").join("missing.txt"),
            optional: true,
            ..VersionFileConfig::default()
        };

        update_all(&[file], &plan(), false).expect("missing optional file is skipped");
    }
}
