//! TOML configuration model. Defaults are conservative so a generated config is safe to inspect before the first release.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub release: ReleaseConfig,
    pub source_release: Option<ReleaseConfig>,
    pub github: GitHubConfig,
    pub prepare_commands: Vec<String>,
    pub files: Vec<VersionFileConfig>,
}

impl Config {
    /// Returns the view used by prepare/finalize/abort commands in a monorepo release.
    ///
    /// The regular `[release]` section describes the public distribution release. The optional
    /// `[source_release]` section describes the source repository branch/tag that is created before
    /// assets are built. When `[source_release]` is omitted, the public release settings are reused
    /// for backwards-compatible single-repository projects.
    pub fn source_view(&self) -> Self {
        let mut config = self.clone();
        if let Some(source_release) = self.source_release.clone() {
            config.release = source_release;
            config.github = GitHubConfig {
                generate_notes: false,
                ..GitHubConfig::default()
            };
            config.source_release = None;
        }
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            release: ReleaseConfig::default(),
            source_release: None,
            github: GitHubConfig::default(),
            prepare_commands: Vec::new(),
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
    pub floating_tags: bool,
    pub latest_tag: bool,
    pub next_tag: bool,
    pub latest_tag_name: String,
    pub next_tag_name: String,
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
            floating_tags: false,
            latest_tag: false,
            next_tag: false,
            latest_tag_name: "latest".to_string(),
            next_tag_name: "next".to_string(),
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
    pub notes_body: String,
    pub notes: NotesConfig,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            target_repository: String::new(),
            source_repository: String::new(),
            source_tag_prefix: String::new(),
            source_tag_suffix: String::new(),
            generate_notes: true,
            notes_body: String::new(),
            notes: NotesConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct NotesConfig {
    pub mode: NotesMode,
    pub include_scopes: Vec<String>,
    pub include_paths: Vec<String>,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NotesMode {
    #[default]
    Github,
    Scoped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct VersionFileConfig {
    pub path: PathBuf,
    pub kind: VersionFileKind,
    pub key: String,
    pub value: String,
    pub value_type: VersionValueType,
    pub search: String,
    pub replace: String,
    pub package: String,
    pub optional: bool,
}

impl VersionFileConfig {
    pub fn cargo_toml() -> Self {
        Self {
            path: PathBuf::from("Cargo.toml"),
            kind: VersionFileKind::Toml,
            key: "package.version".to_string(),
            value: "{version}".to_string(),
            value_type: VersionValueType::default(),
            search: String::new(),
            replace: String::new(),
            package: String::new(),
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
            value_type: VersionValueType::default(),
            search: String::new(),
            replace: String::new(),
            package: String::new(),
            optional: false,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum VersionFileKind {
    Toml,
    Json,
    #[serde(alias = "cargo-lock-package")]
    CargoLockPackage,
    #[default]
    Text,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum VersionValueType {
    #[default]
    String,
    Integer,
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
    value
        .get("release")
        .and_then(|release| release.get("targets"))
        .and_then(toml::Value::as_array)
        .is_some()
}

fn load_datarose_config(
    value: &toml::Value,
    release_target: Option<&str>,
    path: &Path,
) -> Result<Config> {
    let release = value
        .get("release")
        .and_then(toml::Value::as_table)
        .with_context(|| format!("{} is missing [release]", path.display()))?;
    let targets = release
        .get("targets")
        .and_then(toml::Value::as_array)
        .with_context(|| format!("{} is missing [[release.targets]]", path.display()))?;
    let target = select_datarose_target(targets, release_target, path)?;
    let target_table = target
        .as_table()
        .with_context(|| format!("invalid release target in {}", path.display()))?;
    let name = string_field(target_table, "name").with_context(|| {
        format!(
            "release target in {} is missing the required `name` field",
            path.display()
        )
    })?;
    let repository = string_field(target_table, "repository")
        .or_else(|| string_field(target_table, "target_repository"))
        .unwrap_or_else(|| format!("verzly/{name}"));
    let source_repository = string_field(target_table, "source_repository")
        .or_else(|| string_field(release, "source_repository"))
        .unwrap_or_default();
    let target_branch = string_field(release, "target_branch").unwrap_or_else(|| "master".into());
    let source_tag_prefix =
        string_field(target_table, "source_tag_prefix").unwrap_or_else(|| format!("{name}-v"));

    let default_release = || ReleaseConfig {
        target_branch: target_branch.clone(),
        tag_prefix: string_field(target_table, "release_tag_prefix").unwrap_or_else(|| "v".into()),
        name_prefix: string_field(target_table, "release_name_prefix").unwrap_or_default(),
        floating_tags: bool_field(target_table, "floating_tags").unwrap_or(true),
        latest_tag: bool_field(target_table, "latest_tag").unwrap_or(true),
        next_tag: bool_field(target_table, "next_tag").unwrap_or(true),
        ..ReleaseConfig::default()
    };
    let default_source_release = || ReleaseConfig {
        target_branch: target_branch.clone(),
        tag_prefix: source_tag_prefix.clone(),
        name_prefix: format!("{name} v"),
        latest: false,
        ..ReleaseConfig::default()
    };

    let mut config = Config {
        prepare_commands: string_array_field(target_table, "prepare_commands"),
        files: datarose_version_files(target_table, &name)?,
        release: decode_table_field(target_table, "release")?.unwrap_or_else(default_release),
        source_release: datarose_source_release(target_table, default_source_release)?,
        github: decode_table_field(target_table, "github")?.unwrap_or_else(GitHubConfig::default),
    };

    if config.github.target_repository.is_empty() {
        config.github.target_repository = repository;
    }
    if config.github.source_repository.is_empty() {
        config.github.source_repository = source_repository;
    }
    if config.github.source_tag_prefix.is_empty() {
        config.github.source_tag_prefix = source_tag_prefix;
    }
    if string_field(target_table, "generate_notes").is_some() {
        // Ignore string values. This field is supported through [release.targets.github].
    }
    if let Some(value) = bool_field(target_table, "generate_notes") {
        config.github.generate_notes = value;
    } else if !target_table.contains_key("github") {
        config.github.generate_notes = false;
    }
    if config.github.notes_body.is_empty() {
        config.github.notes_body = default_distribution_notes_body();
    }
    if config.github.notes.include_scopes.is_empty() {
        config.github.notes.include_scopes = string_array_field(target_table, "include_scopes");
    }
    if config.github.notes.include_paths.is_empty() {
        config.github.notes.include_paths = string_array_field(target_table, "include_paths");
    }
    if !config.github.notes.include_scopes.is_empty()
        || !config.github.notes.include_paths.is_empty()
    {
        config.github.notes.mode = NotesMode::Scoped;
    }

    Ok(config)
}

fn select_datarose_target<'a>(
    targets: &'a [toml::Value],
    release_target: Option<&str>,
    path: &Path,
) -> Result<&'a toml::Value> {
    if let Some(release_target) = release_target {
        for target in targets {
            let Some(table) = target.as_table() else {
                continue;
            };
            if string_field(table, "name").as_deref() == Some(release_target) {
                return Ok(target);
            }
        }
        anyhow::bail!(
            "release target `{}` was not found in {}",
            release_target,
            path.display()
        );
    }

    if targets.len() == 1 {
        return Ok(&targets[0]);
    }

    anyhow::bail!(
        "{} contains multiple release targets; pass --release-target <name>",
        path.display()
    )
}

fn datarose_version_files(
    target_table: &toml::Table,
    name: &str,
) -> Result<Vec<VersionFileConfig>> {
    if bool_field(target_table, "version_files") == Some(false) {
        return Ok(Vec::new());
    }

    if let Some(files) = target_table.get("files") {
        return files
            .clone()
            .try_into()
            .context("failed to parse release target files");
    }

    let path = string_field(target_table, "version_file")
        .unwrap_or_else(|| format!("crates/{name}/Cargo.toml"));
    Ok(vec![VersionFileConfig {
        path: PathBuf::from(path),
        kind: VersionFileKind::Toml,
        key: string_field(target_table, "version_key").unwrap_or_else(|| "package.version".into()),
        value: string_field(target_table, "version_value").unwrap_or_else(|| "{version}".into()),
        optional: bool_field(target_table, "version_file_optional").unwrap_or(false),
        ..VersionFileConfig::default()
    }])
}

fn datarose_source_release<F>(
    target_table: &toml::Table,
    default_source_release: F,
) -> Result<Option<ReleaseConfig>>
where
    F: FnOnce() -> ReleaseConfig,
{
    match target_table.get("source_release") {
        Some(toml::Value::Boolean(false)) => Ok(None),
        Some(toml::Value::Boolean(true)) => Ok(Some(default_source_release())),
        Some(value @ toml::Value::Table(_)) => value
            .clone()
            .try_into()
            .context("failed to parse `source_release` release target section")
            .map(Some),
        Some(_) => anyhow::bail!(
            "failed to parse `source_release` release target section: expected a boolean or table"
        ),
        None => Ok(Some(default_source_release())),
    }
}

fn decode_table_field<T>(table: &toml::Table, key: &str) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    value
        .clone()
        .try_into()
        .with_context(|| format!("failed to parse `{key}` release target section"))
        .map(Some)
}

fn string_field(table: &toml::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn bool_field(table: &toml::Table, key: &str) -> Option<bool> {
    table.get(key).and_then(toml::Value::as_bool)
}

fn string_array_field(table: &toml::Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn default_distribution_notes_body() -> String {
    r#"This version was developed in `verzly/toolchain`.

Source changes for this package can be reviewed from `{previous_source_tag}` to `{source_tag}`:
{source_compare_url}

The distribution repository contains the public GitHub Action surface and release assets. Source changes and pull requests live in `verzly/toolchain`.
"#
    .into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_view_uses_source_release_without_distribution_github_settings() {
        let config = Config {
            release: ReleaseConfig {
                tag_prefix: "v".to_string(),
                latest: true,
                ..ReleaseConfig::default()
            },
            source_release: Some(ReleaseConfig {
                tag_prefix: "cargo-release-v".to_string(),
                name_prefix: "cargo-release v".to_string(),
                latest: false,
                ..ReleaseConfig::default()
            }),
            github: GitHubConfig {
                target_repository: "verzly/cargo-release".to_string(),
                source_repository: "verzly/toolchain".to_string(),
                generate_notes: true,
                ..GitHubConfig::default()
            },
            prepare_commands: vec!["cargo generate-lockfile".to_string()],
            files: vec![VersionFileConfig {
                path: PathBuf::from("crates/cargo-release/Cargo.toml"),
                ..VersionFileConfig::cargo_toml()
            }],
        };

        let source = config.source_view();

        assert_eq!(source.release.tag_prefix, "cargo-release-v");
        assert_eq!(source.release.name_prefix, "cargo-release v");
        assert!(!source.release.latest);
        assert!(source.github.target_repository.is_empty());
        assert!(source.github.source_repository.is_empty());
        assert!(!source.github.generate_notes);
        assert_eq!(
            source.files[0].path,
            PathBuf::from("crates/cargo-release/Cargo.toml")
        );
        assert_eq!(
            source.prepare_commands,
            vec!["cargo generate-lockfile".to_string()]
        );
    }

    #[test]
    fn source_view_keeps_distribution_settings_when_no_source_release_exists() {
        let config = Config {
            github: GitHubConfig {
                target_repository: "verzly/example".to_string(),
                generate_notes: true,
                ..GitHubConfig::default()
            },
            ..Config::default()
        };

        let source = config.source_view();

        assert_eq!(source.release.tag_prefix, "v");
        assert_eq!(source.github.target_repository, "verzly/example");
        assert!(source.github.generate_notes);
    }
    #[test]
    fn loads_release_target_from_datarose_config() {
        let raw = r#"version = 1

[release]
target_branch = "master"
source_repository = "verzly/toolchain"

[[release.targets]]
name = "repository"
repository = "verzly/repository"
prepare_commands = ["cargo generate-lockfile"]
version_file = "crates/repository/Cargo.toml"
include_scopes = ["repository", "all"]
include_paths = ["crates/repository/"]
"#;
        let value: toml::Value = toml::from_str(raw).unwrap();

        let config =
            load_datarose_config(&value, Some("repository"), Path::new("datarose.toml")).unwrap();
        let source = config.source_view();

        assert_eq!(config.github.target_repository, "verzly/repository");
        assert_eq!(config.github.source_repository, "verzly/toolchain");
        assert_eq!(config.github.source_tag_prefix, "repository-v");
        assert_eq!(config.github.notes.mode, NotesMode::Scoped);
        assert_eq!(config.prepare_commands, vec!["cargo generate-lockfile"]);
        assert_eq!(source.release.tag_prefix, "repository-v");
        assert_eq!(
            source.files[0].path,
            PathBuf::from("crates/repository/Cargo.toml")
        );
    }

    #[test]
    fn loads_datarose_target_with_source_release_disabled() {
        let raw = r#"version = 1

[release]
target_branch = "master"
source_repository = "verzly/toolchain"

[[release.targets]]
name = "toolchain"
repository = "verzly/toolchain"
source_repository = ""
version_files = false
source_tag_prefix = "v"
release_name_prefix = "toolchain v"
source_release = false
generate_notes = true
"#;
        let value: toml::Value = toml::from_str(raw).unwrap();

        let config =
            load_datarose_config(&value, Some("toolchain"), Path::new("datarose.toml")).unwrap();
        let source = config.source_view();

        assert!(config.source_release.is_none());
        assert!(source.source_release.is_none());
        assert!(config.files.is_empty());
        assert_eq!(config.release.tag_prefix, "v");
        assert_eq!(config.release.name_prefix, "toolchain v");
        assert_eq!(config.github.target_repository, "verzly/toolchain");
        assert!(config.github.source_repository.is_empty());
        assert!(config.github.generate_notes);
        assert_eq!(source.github.target_repository, "verzly/toolchain");
        assert_eq!(source.release.tag_prefix, "v");
        assert!(source.files.is_empty());
    }
}
