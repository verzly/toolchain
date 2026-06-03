//! Release build configuration. The config is explicit on purpose: targets and artifact globs should not be guessed.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub project: ProjectConfig,
    pub build: BuildConfig,
    pub artifacts: ArtifactConfig,
    pub targets: BTreeMap<String, TargetConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut targets = BTreeMap::new();
        targets.insert(
            "linux-x64".to_string(),
            TargetConfig::linux_default("my-tool"),
        );
        Self {
            project: ProjectConfig::default(),
            build: BuildConfig::default(),
            artifacts: ArtifactConfig::default(),
            targets,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub binary: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            binary: "my-tool".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct BuildConfig {
    pub out_dir: PathBuf,
    pub default_strategy: Strategy,
    pub container_engine: ContainerEngine,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("dist"),
            default_strategy: Strategy::Host,
            container_engine: ContainerEngine::Podman,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ArtifactConfig {
    pub checksum: bool,
    pub manifest: bool,
    pub name_template: String,
}

impl Default for ArtifactConfig {
    fn default() -> Self {
        Self {
            checksum: true,
            manifest: true,
            name_template: "{binary}-v{version}-{target}{ext}".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TargetConfig {
    pub enabled: bool,
    pub triple: String,
    pub strategy: Strategy,
    pub image: Option<String>,
    pub command: String,
    pub artifacts: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl TargetConfig {
    pub fn linux_default(binary: &str) -> Self {
        Self {
            enabled: true,
            triple: "x86_64-unknown-linux-gnu".to_string(),
            strategy: Strategy::Host,
            image: None,
            command: "cargo build --release --target x86_64-unknown-linux-gnu".to_string(),
            artifacts: vec![format!("target/x86_64-unknown-linux-gnu/release/{binary}")],
            env: BTreeMap::new(),
        }
    }
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self::linux_default("my-tool")
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Strategy {
    Auto,
    #[default]
    Host,
    Container,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContainerEngine {
    Docker,
    #[default]
    Podman,
}

impl ContainerEngine {
    pub fn executable(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

pub fn load(path: &Path) -> Result<Config> {
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
        std::env::temp_dir().join(format!("cargo-release-{name}-{suffix}.toml"))
    }

    #[test]
    fn default_config_is_explicit_and_portable() {
        let config = Config::default();
        let linux = config.targets.get("linux-x64").expect("linux target");

        assert_eq!(config.project.binary, "my-tool");
        assert_eq!(config.build.out_dir, PathBuf::from("dist"));
        assert_eq!(config.build.container_engine, ContainerEngine::Podman);
        assert_eq!(
            config.artifacts.name_template,
            "{binary}-v{version}-{target}{ext}"
        );
        assert_eq!(linux.strategy, Strategy::Host);
        assert!(linux.artifacts[0].contains("target/x86_64-unknown-linux-gnu/release/my-tool"));
    }

    #[test]
    fn write_default_config_refuses_to_overwrite_without_force() {
        let path = temp_path("default");
        write_default_config(&path, false).expect("write config");

        let error = write_default_config(&path, false).expect_err("existing config must fail");
        assert!(error.to_string().contains("config already exists"));

        write_default_config(&path, true).expect("force overwrites config");
    }
}
