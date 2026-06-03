//! Cache configuration. Defaults keep the cache project-local and avoid redirecting Cargo home unless requested.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
        }
    }
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
