//! Build command orchestration for enabled platforms. The command coordinates work; platform behavior stays in config.

use crate::artifacts;
use crate::cli::BuildArgs;
use crate::config::{self, Strategy};
use crate::container;
use crate::manifest;
use crate::process;
use anyhow::Result;

pub fn run(args: BuildArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let project_root = config.project.root.clone();
    let out_dir = config.build.out_dir.clone();

    if !args.dry_run {
        artifacts::prepare_out_dir(&out_dir)?;
    }

    if let Some(command) = config.project.frontend_install.as_ref() {
        let empty_env = std::collections::BTreeMap::new();
        process::shell(command, &empty_env, args.dry_run)?;
    }

    let mut records = Vec::new();
    let mut matched_platform = false;
    for (name, platform) in &config.platforms {
        if !platform.enabled {
            continue;
        }
        if let Some(selected) = args.platform.as_ref() {
            if selected != name {
                continue;
            }
        }
        matched_platform = true;

        let strategy = match platform.strategy {
            Strategy::Auto => config.build.default_strategy,
            other => other,
        };

        println!("building {name} ({strategy:?})");
        match strategy {
            Strategy::Host | Strategy::Auto => {
                process::shell(&platform.command, &platform.env, args.dry_run)?
            }
            Strategy::Container => container::run(
                config.build.container_engine,
                &project_root,
                platform,
                args.dry_run,
            )?,
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

    if let Some(selected_platform) = args.platform.as_ref() {
        if !matched_platform {
            anyhow::bail!("unknown or disabled release platform: {selected_platform}");
        }
    }

    if !args.dry_run && config.artifacts.manifest {
        manifest::write(&out_dir.join("manifest.json"), records)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tauri-release-build-{name}-{suffix}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("datarose.toml");
        std::fs::write(
            &path,
            toml::to_string_pretty(&crate::config::Config::default()).expect("serialize config"),
        )
        .expect("write config");
        path
    }

    #[test]
    fn build_fails_for_unknown_or_disabled_selected_platform() {
        let unknown = run(BuildArgs {
            config: temp_config("unknown"),
            platform: Some("windows".to_string()),
            dry_run: true,
        })
        .expect_err("unknown platform should fail");
        assert!(unknown
            .to_string()
            .contains("unknown or disabled release platform"));

        let disabled = run(BuildArgs {
            config: temp_config("disabled"),
            platform: Some("android".to_string()),
            dry_run: true,
        })
        .expect_err("disabled platform should fail");
        assert!(disabled
            .to_string()
            .contains("unknown or disabled release platform"));
    }
}
