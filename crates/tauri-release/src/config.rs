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
    let value: toml::Value =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;

    if value.get("tauri_release").is_some() {
        return Ok(load_datarose_config(&value));
    }

    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn load_datarose_config(value: &toml::Value) -> Config {
    let mut config = Config::default();
    let Some(root) = value.get("tauri_release") else {
        return config;
    };

    if let Some(project) = root.get("project").and_then(toml::Value::as_table) {
        if let Some(path) = string_field(project, "root") {
            config.project.root = PathBuf::from(path);
        }
        config.project.frontend_install = string_field(project, "frontend_install");
    }

    if let Some(build) = root.get("build").and_then(toml::Value::as_table) {
        if let Some(path) = string_field(build, "out_dir") {
            config.build.out_dir = PathBuf::from(path);
        }
        if let Some(path) = string_field(build, "cache_dir") {
            config.build.cache_dir = PathBuf::from(path);
        }
    }

    config
}

fn string_field(table: &toml::value::Table, key: &str) -> Option<String> {
    table.get(key)?.as_str().map(ToOwned::to_owned)
}

pub fn write_default_config(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!("config already exists: {}", path.display());
    }
    fs::write(path, render_datarose_default_config())?;
    Ok(())
}

fn render_datarose_default_config() -> String {
    r#"version = 1

[tauri_release.project]
root = "."
frontend_install = "aube install"

[tauri_release.build]
out_dir = "dist"
cache_dir = ".cache/tauri-release"
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_config_separates_desktop_and_android_platforms() {
        let config = Config::default();
        let linux = config.platforms.get("linux").expect("linux platform");
        let android = config.platforms.get("android").expect("android platform");

        assert_eq!(config.build.out_dir, PathBuf::from("dist"));
        assert_eq!(
            config.build.cache_dir,
            PathBuf::from(".cache/tauri-release")
        );
        assert_eq!(linux.strategy, Strategy::Host);
        assert!(linux.enabled);
        assert_eq!(android.strategy, Strategy::Container);
        assert!(!android.enabled);
        assert!(android.command.contains("tauri android build"));
    }

    #[test]
    fn container_engine_resolves_executable_name() {
        assert_eq!(ContainerEngine::Docker.executable(), "docker");
        assert_eq!(ContainerEngine::Podman.executable(), "podman");
    }
}
