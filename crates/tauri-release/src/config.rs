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
        platforms.insert("windows".to_string(), PlatformConfig::windows_default());
        platforms.insert("macos".to_string(), PlatformConfig::macos_default());
        platforms.insert("android".to_string(), PlatformConfig::android_default());
        platforms.insert("ios".to_string(), PlatformConfig::ios_default());
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
    pub required_host_os: Option<String>,
    pub required_commands: Vec<String>,
    pub required_env: Vec<String>,
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
            required_host_os: Some("linux".to_string()),
            required_commands: vec!["pnpm".to_string()],
            required_env: Vec::new(),
            command: "pnpm tauri build".to_string(),
            artifacts: vec![
                "src-tauri/target/release/bundle/**/*.deb".to_string(),
                "src-tauri/target/release/bundle/**/*.AppImage".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    pub fn windows_default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Container,
            image: Some("ghcr.io/verzly/tauri-release-windows:latest".to_string()),
            required_host_os: None,
            required_commands: Vec::new(),
            required_env: Vec::new(),
            command: "pnpm tauri build --target x86_64-pc-windows-msvc".to_string(),
            artifacts: vec![
                "src-tauri/target/release/bundle/**/*.msi".to_string(),
                "src-tauri/target/release/bundle/**/*.exe".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    pub fn macos_default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Host,
            image: None,
            required_host_os: Some("macos".to_string()),
            required_commands: vec!["pnpm".to_string(), "xcodebuild".to_string()],
            required_env: Vec::new(),
            command: "pnpm tauri build".to_string(),
            artifacts: vec![
                "src-tauri/target/release/bundle/**/*.dmg".to_string(),
                "src-tauri/target/release/bundle/**/*.app.tar.gz".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    pub fn android_default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Container,
            image: Some("ghcr.io/verzly/tauri-release-android:latest".to_string()),
            required_host_os: None,
            required_commands: Vec::new(),
            required_env: vec![
                "ANDROID_KEYSTORE_PATH".to_string(),
                "ANDROID_KEYSTORE_PASSWORD".to_string(),
                "ANDROID_KEY_ALIAS".to_string(),
                "ANDROID_KEY_PASSWORD".to_string(),
            ],
            command: "pnpm tauri android build --apk --aab".to_string(),
            artifacts: vec![
                "src-tauri/gen/android/app/build/outputs/**/*.apk".to_string(),
                "src-tauri/gen/android/app/build/outputs/**/*.aab".to_string(),
            ],
            env: BTreeMap::new(),
        }
    }

    pub fn ios_default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Host,
            image: None,
            required_host_os: Some("macos".to_string()),
            required_commands: vec!["pnpm".to_string(), "xcodebuild".to_string()],
            required_env: vec![
                "IOS_SIGNING_CERTIFICATE_BASE64".to_string(),
                "IOS_SIGNING_CERTIFICATE_PASSWORD".to_string(),
                "IOS_SIGNING_PROVISIONING_PROFILE_BASE64".to_string(),
                "IOS_SIGNING_KEYCHAIN_PASSWORD".to_string(),
                "APPLE_TEAM_ID".to_string(),
            ],
            command: "pnpm tauri ios build".to_string(),
            artifacts: vec!["src-tauri/gen/apple/build/**/*.ipa".to_string()],
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
        return load_datarose_config(&value);
    }

    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn load_datarose_config(value: &toml::Value) -> Result<Config> {
    let mut config = Config::default();
    let Some(root) = value.get("tauri_release") else {
        return Ok(config);
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
        if let Some(strategy) = string_field(build, "default_strategy") {
            config.build.default_strategy = parse_strategy(&strategy)?;
        }
        if let Some(engine) = string_field(build, "container_engine") {
            config.build.container_engine = parse_container_engine(&engine)?;
        }
    }

    if let Some(artifacts) = root.get("artifacts").and_then(toml::Value::as_table) {
        if let Some(value) = bool_field(artifacts, "checksum") {
            config.artifacts.checksum = value;
        }
        if let Some(value) = bool_field(artifacts, "manifest") {
            config.artifacts.manifest = value;
        }
    }

    if let Some(platforms) = root.get("platforms").and_then(toml::Value::as_table) {
        for (name, value) in platforms {
            let Some(table) = value.as_table() else {
                continue;
            };
            let mut platform = config
                .platforms
                .remove(name)
                .unwrap_or_else(PlatformConfig::default);
            apply_platform_overrides(table, &mut platform)?;
            config.platforms.insert(name.clone(), platform);
        }
    }

    Ok(config)
}

fn string_field(table: &toml::value::Table, key: &str) -> Option<String> {
    table.get(key)?.as_str().map(ToOwned::to_owned)
}

fn optional_string_field(table: &toml::value::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn bool_field(table: &toml::value::Table, key: &str) -> Option<bool> {
    table.get(key)?.as_bool()
}

fn string_array_field(table: &toml::value::Table, key: &str) -> Option<Vec<String>> {
    Some(
        table
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect(),
    )
}

fn string_table_field(table: &toml::value::Table, key: &str) -> Option<BTreeMap<String, String>> {
    Some(
        table
            .get(key)?
            .as_table()?
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
            .collect(),
    )
}

fn apply_platform_overrides(
    table: &toml::value::Table,
    platform: &mut PlatformConfig,
) -> Result<()> {
    if let Some(value) = bool_field(table, "enabled") {
        platform.enabled = value;
    }
    if let Some(value) = string_field(table, "strategy") {
        platform.strategy = parse_strategy(&value)?;
    }
    if table.contains_key("image") {
        platform.image = optional_string_field(table, "image");
    }
    if table.contains_key("required_host_os") {
        platform.required_host_os = optional_string_field(table, "required_host_os");
    }
    if let Some(value) = string_array_field(table, "required_commands") {
        platform.required_commands = value;
    }
    if let Some(value) = string_array_field(table, "required_env") {
        platform.required_env = value;
    }
    if let Some(value) = string_field(table, "command") {
        platform.command = value;
    }
    if let Some(value) = string_array_field(table, "artifacts") {
        platform.artifacts = value;
    }
    if let Some(value) = string_table_field(table, "env") {
        platform.env = value;
    }

    Ok(())
}

fn parse_strategy(value: &str) -> Result<Strategy> {
    match value {
        "auto" => Ok(Strategy::Auto),
        "host" => Ok(Strategy::Host),
        "container" => Ok(Strategy::Container),
        _ => anyhow::bail!("invalid tauri release strategy `{value}`"),
    }
}

fn parse_container_engine(value: &str) -> Result<ContainerEngine> {
    match value {
        "docker" => Ok(ContainerEngine::Docker),
        "podman" => Ok(ContainerEngine::Podman),
        _ => anyhow::bail!("invalid tauri release container engine `{value}`"),
    }
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

[tauri_release.platforms.android]
enabled = false
strategy = "container"
image = "ghcr.io/verzly/tauri-release-android:latest"
required_env = ["ANDROID_KEYSTORE_PATH", "ANDROID_KEYSTORE_PASSWORD", "ANDROID_KEY_ALIAS", "ANDROID_KEY_PASSWORD"]
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
        let ios = config.platforms.get("ios").expect("ios platform");

        assert_eq!(config.build.out_dir, PathBuf::from("dist"));
        assert_eq!(
            config.build.cache_dir,
            PathBuf::from(".cache/tauri-release")
        );
        assert_eq!(linux.strategy, Strategy::Host);
        assert!(linux.enabled);
        assert_eq!(android.strategy, Strategy::Container);
        assert!(!android.enabled);
        assert_eq!(ios.required_host_os.as_deref(), Some("macos"));
        assert!(!ios.enabled);
        assert!(android.command.contains("tauri android build"));
    }

    #[test]
    fn container_engine_resolves_executable_name() {
        assert_eq!(ContainerEngine::Docker.executable(), "docker");
        assert_eq!(ContainerEngine::Podman.executable(), "podman");
    }

    #[test]
    fn datarose_config_accepts_platform_overrides() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("tauri-release-platform-overrides-{suffix}.toml"));
        fs::write(
            &path,
            r#"version = 1

[tauri_release.project]
root = "apps/desktop"
frontend_install = "pnpm install --frozen-lockfile"

[tauri_release.build]
container_engine = "docker"

[tauri_release.platforms.android]
enabled = true
required_env = ["ANDROID_KEYSTORE_PATH"]
env = { ANDROID_KEYSTORE_PATH = "/tmp/release.jks" }
"#,
        )
        .expect("write config");

        let config = load(&path).expect("load config");
        let android = config.platforms.get("android").expect("android platform");

        assert_eq!(config.project.root, PathBuf::from("apps/desktop"));
        assert_eq!(config.build.container_engine, ContainerEngine::Docker);
        assert!(android.enabled);
        assert_eq!(android.required_env, vec!["ANDROID_KEYSTORE_PATH"]);
        assert_eq!(
            android.env.get("ANDROID_KEYSTORE_PATH").map(String::as_str),
            Some("/tmp/release.jks")
        );
    }
}
