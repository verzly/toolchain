//! Deletes a configured release version and repairs moving tags from the remaining releases.

use crate::cli::DeleteArgs;
use crate::config::{self, Config};
use crate::domain::{self, ReleasePlan};
use crate::github::{self, FloatingTagOptions};
use crate::output;
use anyhow::Result;

pub fn run(args: DeleteArgs) -> Result<()> {
    if args.skip_target && args.skip_source {
        anyhow::bail!("use at most one of --skip-target or --skip-source");
    }

    let config = config::load(&args.config, args.release_target.as_deref())?;
    let plan = domain::build_plan(&config, &args.version, None, None, None)?;
    output::print_plan(&plan);

    let mut deleted_any_surface = false;
    if !args.skip_target {
        let repository = target_repository(&plan, args.repository.as_deref(), args.dry_run)?;
        delete_target_surface(&config, &plan, &repository, args.dry_run)?;
        deleted_any_surface = true;
    }

    if !args.skip_source {
        if let Some(repository) = source_repository(&plan, args.source_repository.as_deref()) {
            let target_repository = if args.skip_target {
                None
            } else {
                Some(target_repository(
                    &plan,
                    args.repository.as_deref(),
                    args.dry_run,
                )?)
            };
            let same_surface = target_repository.as_deref() == Some(repository.as_str())
                && plan.github.source_tag == plan.tag;

            if same_surface {
                println!(
                    "source surface resolves to the same repository and tag as the target surface; skipping duplicate delete"
                );
            } else {
                delete_source_surface(&config, &plan, &repository, args.dry_run)?;
                deleted_any_surface = true;
            }
        } else if args.skip_target {
            anyhow::bail!(
                "source deletion was requested, but github.source_repository is not configured"
            );
        } else {
            println!("source repository is not configured; skipping source tag deletion");
        }
    }

    if !deleted_any_surface {
        println!("nothing was deleted");
    }

    Ok(())
}

fn target_repository(
    plan: &ReleasePlan,
    repository_override: Option<&str>,
    dry_run: bool,
) -> Result<String> {
    if let Some(repository) = repository_override {
        return Ok(repository.to_string());
    }

    github::target_repository_for_plan(plan, dry_run)
}

fn source_repository(plan: &ReleasePlan, repository_override: Option<&str>) -> Option<String> {
    repository_override
        .map(ToString::to_string)
        .or_else(|| plan.github.source_repository.clone())
}

fn delete_target_surface(
    config: &Config,
    plan: &ReleasePlan,
    repository: &str,
    dry_run: bool,
) -> Result<()> {
    println!("Deleting target release surface {repository}@{}.", plan.tag);
    github::delete_release_and_tag(repository, &plan.tag, dry_run)?;
    repair_target_floating_tags(config, repository, dry_run)
}

fn delete_source_surface(
    config: &Config,
    plan: &ReleasePlan,
    repository: &str,
    dry_run: bool,
) -> Result<()> {
    println!(
        "Deleting source release surface {repository}@{}.",
        plan.github.source_tag
    );
    github::delete_release_and_tag(repository, &plan.github.source_tag, dry_run)?;
    repair_source_floating_tags(config, repository, dry_run)
}

fn repair_target_floating_tags(config: &Config, repository: &str, dry_run: bool) -> Result<()> {
    repair_floating_tags(
        config,
        repository,
        FloatingTagOptions {
            stable_line_tags: config.release.floating_tags,
            latest_tag: config.release.latest_tag,
            next_tag: config.release.next_tag,
            prune: true,
        },
        dry_run,
    )
}

fn repair_source_floating_tags(config: &Config, repository: &str, dry_run: bool) -> Result<()> {
    let source_config = config.source_view();
    let has_configured_floating_tags = source_config.release.floating_tags
        || source_config.release.latest_tag
        || source_config.release.next_tag;

    if !has_configured_floating_tags {
        println!("source floating tags are not configured; skipping source floating tag repair");
        return Ok(());
    }

    repair_floating_tags(
        &source_config,
        repository,
        FloatingTagOptions {
            stable_line_tags: source_config.release.floating_tags,
            latest_tag: source_config.release.latest_tag,
            next_tag: source_config.release.next_tag,
            prune: true,
        },
        dry_run,
    )
}

fn repair_floating_tags(
    config: &Config,
    repository: &str,
    options: FloatingTagOptions,
    dry_run: bool,
) -> Result<()> {
    println!("Repairing configured floating tags for {repository}.");
    github::ensure_repository_access(repository, dry_run)?;
    github::refresh_highest_floating_tags(
        repository,
        &config.release.tag_prefix,
        &config.release.tag_suffix,
        &config.release.latest_tag_name,
        &config.release.next_tag_name,
        options,
        dry_run,
    )
}
