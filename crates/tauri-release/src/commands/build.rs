//! Build command orchestration for enabled platforms. The command coordinates work; platform behavior stays in config.

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

    if !args.dry_run {
        artifacts::prepare_out_dir(&out_dir)?;
    }

    if let Some(command) = config.project.frontend_install.as_ref() {
        {
            let empty_env = std::collections::BTreeMap::new();
            process::shell(command, &empty_env, args.dry_run)?;
        }
    }

    let mut records = Vec::new();
    for (name, platform) in &config.platforms {
        if !platform.enabled {
            continue;
        }
        if let Some(selected) = args.platform.as_ref() {
            if selected != name {
                continue;
            }
        }

        let strategy = match platform.strategy {
            Strategy::Auto => config.build.default_strategy,
            other => other,
        };

        println!("building {name} ({strategy:?})");
        match strategy {
            Strategy::Host | Strategy::Auto => process::shell(&platform.command, &platform.env, args.dry_run)?,
            Strategy::Container => container::run(config.build.container_engine, &project_root, platform, args.dry_run)?,
        }

        if !args.dry_run {
            records.extend(artifacts::collect(
                name,
                &project_root,
                &out_dir,
                &platform.artifacts,
                config.artifacts.checksum,
            )?);
        }
    }

    if !args.dry_run && config.artifacts.manifest {
        manifest::write(&out_dir.join("manifest.json"), records)?;
    }

    Ok(())
}
