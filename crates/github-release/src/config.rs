//! TOML configuration model. Defaults are conservative so a generated config is safe to inspect before the first release.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub release: ReleaseConfig,
    pub github: GitHubConfig,
    pub files: Vec<VersionFileConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            release: ReleaseConfig::default(),
            github: GitHubConfig::default(),
            files: vec![VersionFileConfig::cargo_toml()],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ReleaseConfig {
    pub target_branch: String,
    pub branch_prefix: String,
    pub tag_prefix: String,
    pub tag_suffix: String,
    pub name_prefix: String,
    pub name_suffix: String,
    pub commit_message: String,
    pub merge_message: String,
    pub cleanup: bool,
    pub latest: bool,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            target_branch: "master".to_string(),
            branch_prefix: "release/".to_string(),
            tag_prefix: "v".to_string(),
            tag_suffix: String::new(),
            name_prefix: String::new(),
            name_suffix: String::new(),
            commit_message: "chore(release): prepare {tag}".to_string(),
            merge_message: "chore(release): merge {tag}".to_string(),
            cleanup: true,
            latest: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GitHubConfig {
    pub target_repository: String,
    pub source_repository: String,
    pub source_tag_prefix: String,
    pub source_tag_suffix: String,
    pub generate_notes: bool,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            target_repository: String::new(),
            source_repository: String::new(),
            source_tag_prefix: String::new(),
            source_tag_suffix: String::new(),
            generate_notes: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct VersionFileConfig {
    pub path: PathBuf,
    pub kind: VersionFileKind,
    pub key: String,
    pub value: String,
    pub search: String,
    pub replace: String,
    pub optional: bool,
}

impl VersionFileConfig {
    pub fn cargo_toml() -> Self {
        Self {
            path: PathBuf::from("Cargo.toml"),
            kind: VersionFileKind::Toml,
            key: "package.version".to_string(),
            value: "{version}".to_string(),
            search: String::new(),
            replace: String::new(),
            optional: false,
        }
    }
}

impl Default for VersionFileConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            kind: VersionFileKind::Text,
            key: String::new(),
            value: "{version}".to_string(),
            search: String::new(),
            replace: String::new(),
            optional: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum VersionFileKind {
    Toml,
    Json,
    Text,
}

impl Default for VersionFileKind {
    fn default() -> Self {
        Self::Text
    }
}

pub fn load(path: &Path) -> Result<Config> {
    let raw = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn write_default_config(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!("config already exists: {}", path.display());
    }

    let config = Config::default();
    let raw = toml::to_string_pretty(&config).context("failed to serialize default config")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
