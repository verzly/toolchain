//! Build command orchestration. This is intentionally thin: plan, execute, collect, then write release metadata.

use anyhow::Result;
use crate::artifacts;
use crate::cli::BuildArgs;
use crate::config::{self, Strategy};
use crate::container;
use crate::manifest;
use crate::process;

pub fn run(args: BuildArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let project_root = config.project.root.clone();
    let out_dir = config.build.out_dir.clone();
    let version = args.version.as_deref().unwrap_or("dev");

    if !args.dry_run {
        artifacts::prepare_out_dir(&out_dir)?;
    }

    let mut records = Vec::new();
    let mut matched_target = false;
    for (name, target) in &config.targets {
        if !target.enabled {
            continue;
        }
        if let Some(selected) = args.target.as_ref() {
            if selected != name {
                continue;
            }
        }
        matched_target = true;

        let strategy = match target.strategy {
            Strategy::Auto => config.build.default_strategy,
            other => other,
        };

        println!("building {name} ({strategy:?})");
        match strategy {
            Strategy::Host | Strategy::Auto => process::shell(&target.command, &target.env, args.dry_run)?,
            Strategy::Container => container::run(config.build.container_engine, &project_root, target, args.dry_run)?,
        }

        if !args.dry_run {
            records.extend(artifacts::collect(
                name,
                &project_root,
                &out_dir,
                &target.artifacts,
                config.artifacts.checksum,
                &config.project.binary,
                version,
                &config.artifacts.name_template,
            )?);
        }
    }

    if args.target.is_some() && !matched_target {
        anyhow::bail!(
            "unknown or disabled release target: {}",
            args.target.as_ref().unwrap()
        );
    }

    if !args.dry_run && config.artifacts.manifest {
        let manifest_name = args
            .target
            .as_ref()
            .map(|target| format!("manifest-{target}.json"))
            .unwrap_or_else(|| "manifest.json".to_string());
        manifest::write(&out_dir.join(manifest_name), records)?;
    }

    Ok(())
}
