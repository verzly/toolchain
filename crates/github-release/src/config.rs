//! TOML configuration model. Defaults are conservative so a generated config is safe to inspect before the first release.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum VersionFileKind {
    Toml,
    Json,
    #[default]
    Text,
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
}
