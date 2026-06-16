//! Maintains moving tags such as v1, v1.2, latest, and next for published releases.

use crate::cli::FloatingTagsArgs;
use crate::config;
use crate::domain;
use crate::github::{self, FloatingTagOptions, FloatingTagUpdate};
use anyhow::Result;

pub fn run(args: FloatingTagsArgs) -> Result<()> {
    let selected_modes =
        u8::from(args.version.is_some()) + u8::from(args.tag.is_some()) + u8::from(args.all);
    if selected_modes != 1 {
        anyhow::bail!("use exactly one of --version, --tag, or --all");
    }
    if args.prune && !args.all {
        anyhow::bail!("--prune can only be used with --all");
    }

    let config = config::load(&args.config, args.release_target.as_deref())?;
    let options = if args.force {
        FloatingTagOptions::force_all()
    } else {
        FloatingTagOptions {
            stable_line_tags: config.release.floating_tags,
            latest_tag: config.release.latest_tag,
            next_tag: config.release.next_tag,
            prune: args.prune,
        }
    };
    let options = FloatingTagOptions {
        prune: args.prune,
        ..options
    };

    if !options.any() {
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
        let version = semver::Version::parse(&plan.version_text)?;
        github::refresh_floating_tags_for_tag(
            FloatingTagUpdate {
                repository,
                full_tag: &plan.tag,
                tag_prefix: &config.release.tag_prefix,
                tag_suffix: &config.release.tag_suffix,
                latest_tag_name: &config.release.latest_tag_name,
                next_tag_name: &config.release.next_tag_name,
                version: &version,
            },
            options,
            args.dry_run,
        )?;
    } else if let Some(tag) = args.tag.as_ref() {
        let Some(version) = github::version_from_tag_for_release(
            tag,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
        ) else {
            println!(
                "skipping floating tags because {tag} does not match a SemVer {}X.Y.Z{} release",
                config.release.tag_prefix, config.release.tag_suffix
            );
            return Ok(());
        };
        github::refresh_floating_tags_for_tag(
            FloatingTagUpdate {
                repository,
                full_tag: tag,
                tag_prefix: &config.release.tag_prefix,
                tag_suffix: &config.release.tag_suffix,
                latest_tag_name: &config.release.latest_tag_name,
                next_tag_name: &config.release.next_tag_name,
                version: &version,
            },
            options,
            args.dry_run,
        )?;
    } else {
        github::refresh_highest_floating_tags(
            repository,
            &config.release.tag_prefix,
            &config.release.tag_suffix,
            &config.release.latest_tag_name,
            &config.release.next_tag_name,
            options,
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
