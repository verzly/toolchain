//! Cache configuration. Defaults keep build output project-local through native Cargo config.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheConfig,
    pub cargo: CargoConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CacheConfig {
    pub dir: PathBuf,
    pub package: String,
    pub redirect_cargo_home: bool,
    pub redirect_gradle: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(".cache"),
            package: "auto".to_string(),
            redirect_cargo_home: false,
            redirect_gradle: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CargoConfig {
    pub target_dir: String,
}

impl Default for CargoConfig {
    fn default() -> Self {
        Self {
            target_dir: "rust/packages/{package}/target".to_string(),
        }
    }
}

pub fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn write_default_config(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!("config already exists: {}", path.display());
    }
    fs::write(path, toml::to_string_pretty(&Config::default())?)?;
    Ok(())
}

pub fn ensure_config(path: &Path, force: bool) -> Result<Config> {
    if !path.exists() || force {
        write_default_config(path, true)?;
    }
    load(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("rust-cache-{name}-{suffix}.toml"))
    }

    #[test]
    fn missing_config_loads_safe_defaults() {
        let config = load(&temp_path("missing")).expect("load default config");

        assert_eq!(config.cache.dir, PathBuf::from(".cache"));
        assert_eq!(config.cache.package, "auto");
        assert!(!config.cache.redirect_cargo_home);
        assert!(config.cache.redirect_gradle);
        assert_eq!(config.cargo.target_dir, "rust/packages/{package}/target");
    }

    #[test]
    fn write_default_config_refuses_existing_file_without_force() {
        let path = temp_path("default");
        write_default_config(&path, false).expect("write config");

        let error = write_default_config(&path, false).expect_err("existing config must fail");
        assert!(error.to_string().contains("config already exists"));

        write_default_config(&path, true).expect("force overwrite");
    }

    #[test]
    fn ensure_config_uses_existing_config_without_overwriting() {
        let path = temp_path("existing");
        fs::write(
            &path,
            "[cache]\npackage = \"demo\"\n\n[cargo]\ntarget_dir = \"rust/{package}/target\"\n",
        )
        .expect("write config");

        let config = ensure_config(&path, false).expect("load existing config");

        assert_eq!(config.cache.package, "demo");
        assert_eq!(config.cargo.target_dir, "rust/{package}/target");
    }
}
