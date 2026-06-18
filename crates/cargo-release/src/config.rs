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
    pub required_env: Vec<String>,
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
            required_env: Vec::new(),
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

pub fn load(path: &Path, release_target: Option<&str>) -> Result<Config> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;

    if is_datarose_config(&value) {
        return load_datarose_config(&value, release_target, path);
    }

    if release_target.is_some() {
        anyhow::bail!(
            "--release-target can only be used with datarose.toml style configs: {}",
            path.display()
        );
    }

    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn is_datarose_config(value: &toml::Value) -> bool {
    value.get("release").is_some() || value.get("cargo_release").is_some()
}

fn load_datarose_config(
    value: &toml::Value,
    release_target: Option<&str>,
    path: &Path,
) -> Result<Config> {
    let target = select_release_target(value, release_target, path)?;
    let binary = string_field(target, "cargo_binary")
        .or_else(|| string_field(target, "binary"))
        .or_else(|| string_field(target, "name"))
        .unwrap_or_else(|| "my-tool".to_string());
    let out_dir = string_field(target, "cargo_out_dir")
        .or_else(|| string_field(target, "out_dir"))
        .unwrap_or_else(|| format!("dist/{binary}"));
    let root = string_field(target, "cargo_root")
        .or_else(|| string_field(target, "root"))
        .unwrap_or_else(|| ".".to_string());
    let package = string_field(target, "cargo_package").unwrap_or_else(|| binary.clone());
    let enabled_targets = string_array_field(target, "cargo_targets").unwrap_or_else(|| {
        vec![
            "linux-x64".to_string(),
            "macos-x64".to_string(),
            "macos-arm64".to_string(),
            "windows-x64".to_string(),
        ]
    });

    let mut config = Config {
        project: ProjectConfig {
            root: PathBuf::from(root),
            binary: binary.clone(),
        },
        build: BuildConfig {
            out_dir: PathBuf::from(out_dir),
            ..BuildConfig::default()
        },
        artifacts: ArtifactConfig::default(),
        targets: BTreeMap::new(),
    };

    for target_name in enabled_targets {
        if let Some(target_config) = datarose_cargo_target(&binary, &package, &target_name) {
            config.targets.insert(target_name, target_config);
        }
    }

    apply_cargo_release_overrides(value, &mut config)?;

    if config.targets.is_empty() {
        anyhow::bail!(
            "{} has no enabled cargo release targets for {}",
            path.display(),
            binary
        );
    }

    Ok(config)
}

fn apply_cargo_release_overrides(value: &toml::Value, config: &mut Config) -> Result<()> {
    let Some(root) = value.get("cargo_release") else {
        return Ok(());
    };

    if let Some(project) = root.get("project").and_then(toml::Value::as_table) {
        if let Some(path) = string_field(project, "root") {
            config.project.root = PathBuf::from(path);
        }
        if let Some(binary) = string_field(project, "binary") {
            config.project.binary = binary;
        }
    }

    if let Some(build) = root.get("build").and_then(toml::Value::as_table) {
        if let Some(path) = string_field(build, "out_dir") {
            config.build.out_dir = PathBuf::from(path);
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
        if let Some(template) = string_field(artifacts, "name_template") {
            config.artifacts.name_template = template;
        }
    }

    let Some(targets) = root.get("targets").and_then(toml::Value::as_table) else {
        return Ok(());
    };

    for (name, value) in targets {
        let Some(table) = value.as_table() else {
            continue;
        };
        let mut target = config
            .targets
            .remove(name)
            .unwrap_or_else(|| TargetConfig::linux_default(&config.project.binary));
        apply_target_overrides(name, table, &mut target)?;
        config.targets.insert(name.clone(), target);
    }

    Ok(())
}

fn apply_target_overrides(
    name: &str,
    table: &toml::value::Table,
    target: &mut TargetConfig,
) -> Result<()> {
    if let Some(value) = bool_field(table, "enabled") {
        target.enabled = value;
    }
    if let Some(value) = string_field(table, "triple") {
        target.triple = value;
    }
    if let Some(value) = string_field(table, "strategy") {
        target.strategy = parse_strategy(&value)?;
    }
    if table.contains_key("image") {
        target.image = optional_string_field(table, "image");
    }
    if let Some(value) = string_field(table, "command") {
        target.command = value;
    }
    if let Some(value) = string_array_field(table, "artifacts") {
        target.artifacts = value;
    }
    if let Some(value) = string_table_field(table, "env") {
        target.env = value;
    }
    if let Some(value) = string_array_field(table, "required_env") {
        target.required_env = value;
    }

    if target.strategy == Strategy::Container && target.image.is_none() && target.enabled {
        anyhow::bail!("cargo release target `{name}` uses container strategy but has no image");
    }

    Ok(())
}

fn select_release_target<'a>(
    value: &'a toml::Value,
    release_target: Option<&str>,
    path: &Path,
) -> Result<&'a toml::value::Table> {
    let targets = value
        .get("release")
        .and_then(|release| release.get("targets"))
        .and_then(toml::Value::as_array)
        .with_context(|| format!("{} is missing [[release.targets]]", path.display()))?;

    if let Some(release_target) = release_target {
        for target in targets {
            let Some(table) = target.as_table() else {
                continue;
            };
            if string_field(table, "name").as_deref() == Some(release_target) {
                return Ok(table);
            }
        }
        anyhow::bail!(
            "{} has no release target named {release_target}",
            path.display()
        );
    }

    if targets.len() == 1 {
        return targets[0]
            .as_table()
            .with_context(|| format!("{} has an invalid release target", path.display()));
    }

    anyhow::bail!(
        "{} contains multiple release targets; pass --release-target <name>",
        path.display()
    );
}

fn datarose_cargo_target(binary: &str, package: &str, target_name: &str) -> Option<TargetConfig> {
    let (triple, executable) = match target_name {
        "linux-x64" => ("x86_64-unknown-linux-gnu", binary.to_string()),
        "macos-x64" => ("x86_64-apple-darwin", binary.to_string()),
        "macos-arm64" => ("aarch64-apple-darwin", binary.to_string()),
        "windows-x64" => ("x86_64-pc-windows-msvc", format!("{binary}.exe")),
        _ => return None,
    };

    Some(TargetConfig {
        enabled: true,
        triple: triple.to_string(),
        strategy: Strategy::Host,
        image: None,
        command: format!("cargo build --release -p {package}"),
        artifacts: vec![format!(
            ".cache/rust/packages/toolchain/target/release/{executable}"
        )],
        env: BTreeMap::new(),
        required_env: Vec::new(),
    })
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

fn parse_strategy(value: &str) -> Result<Strategy> {
    match value {
        "auto" => Ok(Strategy::Auto),
        "host" => Ok(Strategy::Host),
        "container" => Ok(Strategy::Container),
        _ => anyhow::bail!("invalid cargo release strategy `{value}`"),
    }
}

fn parse_container_engine(value: &str) -> Result<ContainerEngine> {
    match value {
        "docker" => Ok(ContainerEngine::Docker),
        "podman" => Ok(ContainerEngine::Podman),
        _ => anyhow::bail!("invalid cargo release container engine `{value}`"),
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

[release]
enabled = true
target_branch = "master"
release_all = false

[[release.targets]]
name = "my-tool"
repository = "owner/my-tool"
cargo_binary = "my-tool"
cargo_package = "my-tool"
cargo_out_dir = "dist/my-tool"
cargo_targets = ["linux-x64", "macos-x64", "macos-arm64", "windows-x64"]
prepare_commands = ["cargo generate-lockfile"]
version_file = "Cargo.toml"
version_key = "package.version"
version_value = "{version}"
include_scopes = ["my-tool", "all"]
include_paths = ["."]
"#
    .to_string()
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

    #[test]
    fn datarose_config_accepts_container_target_overrides() {
        let path = temp_path("datarose-container");
        fs::write(
            &path,
            r#"version = 1

[release]
enabled = true

[[release.targets]]
name = "demo"
repository = "acme/demo"
cargo_binary = "demo"
cargo_package = "demo"
cargo_out_dir = "dist/demo"
cargo_targets = ["linux-x64", "windows-x64"]

[cargo_release.build]
container_engine = "docker"
default_strategy = "container"

[cargo_release.targets.windows-x64]
strategy = "container"
image = "ghcr.io/acme/windows-cross:latest"
command = "cargo build --release -p demo --target x86_64-pc-windows-gnu"
artifacts = ["target/x86_64-pc-windows-gnu/release/demo.exe"]
required_env = ["WINDOWS_SDK_READY"]
"#,
        )
        .expect("write datarose config");

        let config = load(&path, Some("demo")).expect("load datarose config");
        let windows = config.targets.get("windows-x64").expect("windows target");

        assert_eq!(config.build.container_engine, ContainerEngine::Docker);
        assert_eq!(config.build.default_strategy, Strategy::Container);
        assert_eq!(windows.strategy, Strategy::Container);
        assert_eq!(
            windows.image.as_deref(),
            Some("ghcr.io/acme/windows-cross:latest")
        );
        assert!(windows.command.contains("x86_64-pc-windows-gnu"));
        assert_eq!(windows.required_env, vec!["WINDOWS_SDK_READY"]);
    }
}
