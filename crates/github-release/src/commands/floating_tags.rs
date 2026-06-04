//! Maintains stable major/minor floating tags such as v1 and v1.2 for published releases.

use crate::cli::FloatingTagsArgs;
use crate::config;
use crate::domain;
use crate::github;
use anyhow::Result;

pub fn run(args: FloatingTagsArgs) -> Result<()> {
    let selected_modes =
        u8::from(args.version.is_some()) + u8::from(args.tag.is_some()) + u8::from(args.all);
    if selected_modes != 1 {
        anyhow::bail!("use exactly one of --version, --tag, or --all");
    }

    let config = config::load(&args.config)?;
    if !config.release.floating_tags && !args.force {
        println!(
            "floating tags are disabled in {}; skipping",
            args.config.display()
        );
        return Ok(());
    }

    let Some(repository) = args
        .repository
        .as_deref()
        .or_else(|| non_empty(&config.github.target_repository))
    else {
        anyhow::bail!(
            "github.target_repository or --repository is required for floating tag updates"
        );
    };

    if let Some(version) = args.version.as_ref() {
        let plan = domain::build_plan(&config, version, None, None, None)?;
        let Some(version) = github::stable_version_from_tag(
            &plan.tag,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
        ) else {
            println!(
                "skipping floating tags for non-stable release tag {}",
                plan.tag
            );
            return Ok(());
        };
        github::refresh_floating_tags_for_tag(
            repository,
            &plan.tag,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
            &version,
            args.dry_run,
        )?;
    } else if let Some(tag) = args.tag.as_ref() {
        let Some(version) = github::stable_version_from_tag(
            tag,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
        ) else {
            println!(
                "skipping floating tags because {tag} does not match a stable {}X.Y.Z{} release",
                config.release.tag_prefix, config.release.tag_suffix
            );
            return Ok(());
        };
        github::refresh_floating_tags_for_tag(
            repository,
            tag,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
            &version,
            args.dry_run,
        )?;
    } else {
        github::refresh_highest_floating_tags(
            repository,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
            args.dry_run,
        )?;
    }

    Ok(())
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
