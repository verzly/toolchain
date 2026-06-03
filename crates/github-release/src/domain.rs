//! Pure release planning logic. This module resolves names, branches, repositories, and prerelease state without touching Git or GitHub.

use crate::cli::PrereleaseMode;
use crate::config::Config;
use anyhow::{Context, Result};
use semver::Version;

#[derive(Clone, Debug)]
pub struct ReleasePlan {
    pub version_text: String,
    pub tag: String,
    pub release_name: String,
    pub target_branch: String,
    pub release_branch: String,
    pub prerelease: bool,
    pub latest: bool,
    pub commit_message: String,
    pub merge_message: String,
    pub github: GitHubPlan,
}

#[derive(Clone, Debug)]
pub struct GitHubPlan {
    pub target_repository: Option<String>,
    pub source_repository: Option<String>,
    pub source_tag: String,
    pub generate_notes: bool,
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
        release_name,
        target_branch,
        release_branch,
        prerelease,
        latest: config.release.latest,
        commit_message: render_template(&config.release.commit_message, &tag, clean_version),
        merge_message: render_template(&config.release.merge_message, &tag, clean_version),
        github: GitHubPlan {
            target_repository,
            source_repository,
            source_tag,
            generate_notes: config.github.generate_notes,
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
