//! Tauri release configuration. Platform support is explicit because desktop and mobile builds have different constraints.

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
    pub platforms: BTreeMap<String, PlatformConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut platforms = BTreeMap::new();
        platforms.insert("linux".to_string(), PlatformConfig::linux_default());
        platforms.insert("android".to_string(), PlatformConfig::android_default());
        Self {
            project: ProjectConfig::default(),
            build: BuildConfig::default(),
            artifacts: ArtifactConfig::default(),
            platforms,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub frontend_install: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            frontend_install: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct BuildConfig {
    pub out_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub default_strategy: Strategy,
    pub container_engine: ContainerEngine,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("dist"),
            cache_dir: PathBuf::from(".cache/tauri-release"),
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
}

impl Default for ArtifactConfig {
    fn default() -> Self {
        Self {
            checksum: true,
            manifest: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct PlatformConfig {
    pub enabled: bool,
    pub strategy: Strategy,
    pub image: Option<String>,
    pub command: String,
    pub artifacts: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl PlatformConfig {
    pub fn linux_default() -> Self {
        Self {
            enabled: true,
            strategy: Strategy::Host,
            image: None,
            command: "pnpm tauri build".to_string(),
            artifacts: vec![
                "src-tauri/target/release/bundle/**/*.deb".to_string(),
                "src-tauri/target/release/bundle/**/*.AppImage".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    pub fn android_default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Container,
            image: Some("ghcr.io/verzly/tauri-release-android:latest".to_string()),
            command: "pnpm tauri android build --apk --aab".to_string(),
            artifacts: vec![
                "src-tauri/gen/android/app/build/outputs/**/*.apk".to_string(),
                "src-tauri/gen/android/app/build/outputs/**/*.aab".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self::linux_default()
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Strategy {
    Auto,
    Host,
    Container,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::Host
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContainerEngine {
    Docker,
    Podman,
}

impl Default for ContainerEngine {
    fn default() -> Self {
        Self::Podman
    }
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
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn write_default_config(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!("config already exists: {}", path.display());
    }
    fs::write(path, toml::to_string_pretty(&Config::default())?)?;
    Ok(())
}
