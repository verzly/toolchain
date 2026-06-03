//! Native Cargo configuration writer. This is what makes plain `cargo build` use the configured cache path.

use crate::config::Config;
use crate::workspace::{self, Workspace};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

pub fn write_workspace_config(config: &Config, force: bool) -> Result<PathBuf> {
    let workspace = workspace::detect()?;
    write_workspace_config_for(config, &workspace, force)
}

pub fn write_workspace_config_for(
    config: &Config,
    workspace: &Workspace,
    force: bool,
) -> Result<PathBuf> {
    let path = cargo_config_path(&workspace.root);
    let target_dir = target_dir_relative(config, workspace);
    let target_dir = normalize_path(&target_dir);
    let mut root = read_existing_config(&path)?;
    let table = root
        .as_table_mut()
        .context("Cargo config root must be a TOML table")?;
    let build = table
        .entry("build".to_string())
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .context("Cargo config [build] must be a TOML table")?;

    if let Some(existing) = build.get("target-dir").and_then(Value::as_str) {
        if existing != target_dir && !force {
            anyhow::bail!(
                "Cargo target-dir is already configured as {existing:?}; rerun with --force to set it to {target_dir:?}"
            );
        }
    }

    build.insert("target-dir".to_string(), Value::String(target_dir));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, toml::to_string_pretty(&root)?)?;
    Ok(path)
}

pub fn target_dir_relative(config: &Config, workspace: &Workspace) -> PathBuf {
    let package = package_key(config, workspace);
    let target_dir = config.cargo.target_dir.replace("{package}", &package);
    let target_dir = PathBuf::from(target_dir);

    if target_dir.is_absolute() {
        target_dir
    } else {
        config.cache.dir.join(target_dir)
    }
}

pub fn target_dir_absolute(config: &Config, workspace: &Workspace) -> PathBuf {
    let target_dir = target_dir_relative(config, workspace);
    if target_dir.is_absolute() {
        target_dir
    } else {
        workspace.root.join(target_dir)
    }
}

pub fn package_key(config: &Config, workspace: &Workspace) -> String {
    if config.cache.package != "auto" {
        return config.cache.package.clone();
    }

    workspace
        .package
        .clone()
        .or_else(|| {
            workspace
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "workspace".to_string())
}

fn cargo_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".cargo").join("config.toml")
}

fn read_existing_config(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Table(Table::new()));
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read Cargo config {}", path.display()))?;
    let table = toml::from_str::<Table>(&raw)
        .with_context(|| format!("failed to parse Cargo config {}", path.display()))?;
    Ok(Value::Table(table))
}

fn normalize_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheConfig, CargoConfig, Config};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-cache-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn workspace(root: PathBuf) -> Workspace {
        Workspace {
            root,
            package: Some("demo".to_string()),
        }
    }

    #[test]
    fn resolves_target_dir_from_cache_and_package_template() {
        let config = Config {
            cache: CacheConfig {
                dir: PathBuf::from(".cache-test"),
                package: "toolchain".to_string(),
                redirect_cargo_home: false,
                redirect_gradle: false,
            },
            cargo: CargoConfig {
                target_dir: "rust/packages/{package}/target".to_string(),
            },
        };
        let workspace = workspace(temp_dir("resolve"));

        assert_eq!(
            target_dir_relative(&config, &workspace),
            PathBuf::from(".cache-test/rust/packages/toolchain/target")
        );
    }

    #[test]
    fn writes_native_cargo_target_dir_config() {
        let root = temp_dir("write");
        let workspace = workspace(root.clone());
        let path = write_workspace_config_for(&Config::default(), &workspace, false)
            .expect("write Cargo config");

        assert_eq!(path, root.join(".cargo/config.toml"));
        assert!(fs::read_to_string(path)
            .expect("read Cargo config")
            .contains("target-dir = \".cache/rust/packages/demo/target\""));
    }

    #[test]
    fn refuses_to_replace_conflicting_target_dir_without_force() {
        let root = temp_dir("conflict");
        let workspace = workspace(root.clone());
        fs::create_dir_all(root.join(".cargo")).expect("create .cargo");
        let config_path = root.join(".cargo/config.toml");
        fs::write(
            &config_path,
            r#"[build]
"target-dir" = "custom-target"
"#,
        )
        .expect("write existing Cargo config");

        let result = write_workspace_config_for(&Config::default(), &workspace, false);
        assert!(result.is_err(), "conflicting config must fail");
        assert!(fs::read_to_string(&config_path)
            .expect("read unchanged Cargo config")
            .contains("custom-target"));

        write_workspace_config_for(&Config::default(), &workspace, true)
            .expect("force replaces target dir");
        let updated = fs::read_to_string(config_path).expect("read updated Cargo config");
        assert!(updated.contains(".cache/rust/packages/demo/target"));
        assert!(!updated.contains("custom-target"));
    }
}
