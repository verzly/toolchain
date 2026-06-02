//! Pure release planning logic. This module resolves names, branches, and prerelease state without touching Git or GitHub.

use anyhow::{Context, Result};
use semver::Version;

use crate::cli::PrereleaseMode;
use crate::config::Config;

#[derive(Clone, Debug)]
pub struct ReleasePlan {
    pub version: Version,
    pub version_text: String,
    pub tag: String,
    pub release_name: String,
    pub target_branch: String,
    pub release_branch: String,
    pub prerelease: bool,
    pub commit_message: String,
    pub merge_message: String,
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

    Ok(ReleasePlan {
        version,
        version_text,
        tag: tag.clone(),
        release_name,
        target_branch,
        release_branch,
        prerelease,
        commit_message: render_template(&config.release.commit_message, &tag, clean_version),
        merge_message: render_template(&config.release.merge_message, &tag, clean_version),
    })
}

pub fn render_template(template: &str, tag: &str, version: &str) -> String {
    template.replace("{tag}", tag).replace("{version}", version)
}
