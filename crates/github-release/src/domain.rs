//! Pure release planning logic. This module resolves names, branches, repositories, and prerelease state without touching Git or GitHub.

use crate::cli::PrereleaseMode;
use crate::config::{Config, NotesMode};
use anyhow::{Context, Result};
use semver::Version;

#[derive(Clone, Debug)]
pub struct ReleasePlan {
    pub version_text: String,
    pub tag: String,
    pub tag_prefix: String,
    pub tag_suffix: String,
    pub release_name: String,
    pub target_branch: String,
    pub release_branch: String,
    pub prerelease: bool,
    pub latest: bool,
    pub commit_message: String,
    pub merge_message: String,
    pub floating_tags: bool,
    pub github: GitHubPlan,
}

#[derive(Clone, Debug)]
pub struct GitHubPlan {
    pub target_repository: Option<String>,
    pub source_repository: Option<String>,
    pub source_tag: String,
    pub source_tag_prefix: String,
    pub source_tag_suffix: String,
    pub generate_notes: bool,
    pub notes_body: String,
    pub notes: NotesPlan,
}

#[derive(Clone, Debug)]
pub struct NotesPlan {
    pub mode: NotesMode,
    pub include_scopes: Vec<String>,
    pub include_paths: Vec<String>,
}

pub fn build_plan(
    config: &Config,
    version_text: &str,
    target_override: Option<&str>,
    release_branch_override: Option<&str>,
    prerelease_mode: Option<PrereleaseMode>,
) -> Result<ReleasePlan> {
    let clean_version = version_text.strip_prefix('v').unwrap_or(version_text);
    let version = Version::parse(clean_version)
        .with_context(|| format!("invalid SemVer version: {version_text}"))?;
    let version_text = version.to_string();
    let tag = format!(
        "{}{}{}",
        config.release.tag_prefix, version_text, config.release.tag_suffix
    );
    let name_prefix = if config.release.name_prefix.is_empty() {
        &config.release.tag_prefix
    } else {
        &config.release.name_prefix
    };
    let name_suffix = if config.release.name_suffix.is_empty() {
        &config.release.tag_suffix
    } else {
        &config.release.name_suffix
    };
    let release_name = format!("{}{}{}", name_prefix, version_text, name_suffix);
    let target_branch = target_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| config.release.target_branch.clone());
    let release_branch = release_branch_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{}{}", config.release.branch_prefix, tag));
    let prerelease = match prerelease_mode.unwrap_or(PrereleaseMode::Auto) {
        PrereleaseMode::Auto => !version.pre.is_empty(),
        PrereleaseMode::True => true,
        PrereleaseMode::False => false,
    };

    let target_repository = non_empty(&config.github.target_repository);
    let source_repository = non_empty(&config.github.source_repository);
    let source_tag = if source_repository.is_some() {
        format!(
            "{}{}{}",
            config.github.source_tag_prefix, version_text, config.github.source_tag_suffix
        )
    } else {
        tag.clone()
    };

    Ok(ReleasePlan {
        version_text,
        tag: tag.clone(),
        tag_prefix: config.release.tag_prefix.clone(),
        tag_suffix: config.release.tag_suffix.clone(),
        release_name,
        target_branch,
        release_branch,
        prerelease,
        latest: config.release.latest,
        commit_message: render_template(&config.release.commit_message, &tag, clean_version),
        merge_message: render_template(&config.release.merge_message, &tag, clean_version),
        floating_tags: config.release.floating_tags,
        github: GitHubPlan {
            target_repository,
            source_repository,
            source_tag,
            source_tag_prefix: config.github.source_tag_prefix.clone(),
            source_tag_suffix: config.github.source_tag_suffix.clone(),
            generate_notes: config.github.generate_notes,
            notes_body: config.github.notes_body.clone(),
            notes: NotesPlan {
                mode: config.github.notes.mode,
                include_scopes: config.github.notes.include_scopes.clone(),
                include_paths: config.github.notes.include_paths.clone(),
            },
        },
    })
}

pub fn render_template(template: &str, tag: &str, version: &str) -> String {
    template.replace("{tag}", tag).replace("{version}", version)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::PrereleaseMode;
    use crate::config::{Config, GitHubConfig, NotesConfig, NotesMode, ReleaseConfig};

    fn config() -> Config {
        Config {
            release: ReleaseConfig {
                tag_prefix: "tool-v".to_string(),
                tag_suffix: "-dist".to_string(),
                branch_prefix: "release/".to_string(),
                commit_message: "release {tag} from {version}".to_string(),
                merge_message: "merge {tag}".to_string(),
                ..ReleaseConfig::default()
            },
            github: GitHubConfig {
                source_repository: "verzly/toolchain".to_string(),
                source_tag_prefix: "tool-v".to_string(),
                notes: NotesConfig {
                    mode: NotesMode::Scoped,
                    include_scopes: vec!["tool".to_string(), "all".to_string()],
                    include_paths: vec!["crates/tool/".to_string()],
                },
                ..GitHubConfig::default()
            },
            ..Config::default()
        }
    }

    #[test]
    fn builds_release_plan_with_prefixes_suffixes_and_source_tag() {
        let plan = build_plan(&config(), "v1.2.3", None, None, None).expect("valid plan");

        assert_eq!(plan.version_text, "1.2.3");
        assert_eq!(plan.tag, "tool-v1.2.3-dist");
        assert_eq!(plan.release_name, "tool-v1.2.3-dist");
        assert_eq!(plan.release_branch, "release/tool-v1.2.3-dist");
        assert_eq!(plan.github.source_tag, "tool-v1.2.3");
        assert_eq!(plan.commit_message, "release tool-v1.2.3-dist from 1.2.3");
        assert_eq!(plan.merge_message, "merge tool-v1.2.3-dist");
    }

    #[test]
    fn detects_prerelease_automatically_and_accepts_overrides() {
        let config = config();

        let auto = build_plan(&config, "1.2.3-rc.1", None, None, None).expect("valid plan");
        let forced_false = build_plan(
            &config,
            "1.2.3-rc.1",
            None,
            None,
            Some(PrereleaseMode::False),
        )
        .expect("valid plan");
        let forced_true = build_plan(&config, "1.2.3", None, None, Some(PrereleaseMode::True))
            .expect("valid plan");

        assert!(auto.prerelease);
        assert!(!forced_false.prerelease);
        assert!(forced_true.prerelease);
    }

    #[test]
    fn rejects_invalid_semver_versions() {
        let error = build_plan(&config(), "not-a-version", None, None, None)
            .expect_err("invalid versions must fail");

        assert!(error.to_string().contains("invalid SemVer version"));
    }
}
