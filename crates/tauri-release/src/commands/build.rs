//! Build command orchestration for enabled platforms. The command coordinates work; platform behavior stays in config.

use crate::artifacts;
use crate::cli::BuildArgs;
use crate::config::{self, Strategy};
use crate::container;
use crate::manifest;
use crate::process;
use anyhow::Result;
use std::env;

pub fn run(args: BuildArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let project_root = config.project.root.clone();
    let out_dir = config.build.out_dir.clone();

    if !args.dry_run {
        artifacts::prepare_out_dir(&out_dir)?;
    }

    let mut records = Vec::new();
    let mut skipped = Vec::new();
    let mut matched_platform = false;
    let mut frontend_installed = false;
    for (name, platform) in &config.platforms {
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

        let skip = skip_reasons(name, platform, strategy, config.build.container_engine);
        if !skip.reasons.is_empty() {
            println!("skipping {name}:");
            for reason in &skip.reasons {
                println!("  - {reason}");
            }
            println!("  next steps:");
            for step in &skip.next_steps {
                println!("  - {step}");
            }
            skipped.push(skip);
            continue;
        }

        if !frontend_installed {
            if let Some(command) = config.project.frontend_install.as_ref() {
                let empty_env = std::collections::BTreeMap::new();
                process::shell(command, &empty_env, args.dry_run)?;
            }
            frontend_installed = true;
        }

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
            match artifacts::collect(
                name,
                &project_root,
                &out_dir,
                &platform.artifacts,
                config.artifacts.checksum,
            ) {
                Ok(platform_records) => records.extend(platform_records),
                Err(error) => {
                    let skip = artifacts::SkippedPlatform {
                        platform: name.to_string(),
                        reasons: vec![error.to_string()],
                        next_steps: vec![
                            "Check the platform artifact globs against the real Tauri output paths."
                                .to_string(),
                            "Enable this platform only after the configured command produces those files."
                                .to_string(),
                        ],
                    };
                    println!("skipping {name}:");
                    for reason in &skip.reasons {
                        println!("  - {reason}");
                    }
                    println!("  next steps:");
                    for step in &skip.next_steps {
                        println!("  - {step}");
                    }
                    skipped.push(skip);
                }
            }
        }
    }

    if let Some(selected_platform) = args.platform.as_ref() {
        if !matched_platform {
            anyhow::bail!("unknown release platform: {selected_platform}");
        }
    }

    if !args.dry_run && config.artifacts.manifest {
        manifest::write(&out_dir.join("manifest.json"), records, skipped)?;
    }

    Ok(())
}

fn skip_reasons(
    name: &str,
    platform: &crate::config::PlatformConfig,
    strategy: Strategy,
    engine: crate::config::ContainerEngine,
) -> artifacts::SkippedPlatform {
    let mut reasons = Vec::new();
    let mut next_steps = Vec::new();

    if !platform.enabled {
        reasons.push("platform is disabled in configuration".to_string());
        next_steps.push(format!(
            "Set `tauri_release.platforms.{name}.enabled = true`."
        ));
        return artifacts::SkippedPlatform {
            platform: name.to_string(),
            reasons,
            next_steps,
        };
    }

    if strategy != Strategy::Container {
        if let Some(required) = platform.required_host_os.as_deref() {
            let current = current_host_os();
            if required != current {
                reasons.push(format!(
                    "platform requires {required} host but current host is {current}"
                ));
                next_steps.push(format!(
                "Run this platform on a {required} runner or switch it to a supported container strategy."
            ));
            }
        }
    }

    if strategy == Strategy::Container {
        if platform.image.is_none() {
            reasons.push("container strategy requires an image".to_string());
            next_steps.push(format!(
                "Set `tauri_release.platforms.{name}.image` to a Docker/Podman image."
            ));
        }
        if !process::available(engine.executable()) {
            reasons.push(format!(
                "container engine `{}` is not available",
                engine.executable()
            ));
            next_steps.push(format!(
                "Install {} or configure `tauri_release.build.container_engine`.",
                engine.executable()
            ));
        }
    }

    if strategy != Strategy::Container {
        for command in &platform.required_commands {
            if !process::available(command) {
                reasons.push(format!("required command `{command}` is not available"));
                next_steps.push(format!(
                    "Install `{command}` on the runner before enabling `{name}`."
                ));
            }
        }
    }

    let missing_env = missing_required_env(platform);
    if !missing_env.is_empty() {
        reasons.push(format!(
            "missing required environment variables: {}",
            missing_env.join(", ")
        ));
        next_steps.push(format!(
            "Provide the missing values through CI secrets or `tauri_release.platforms.{name}.env`."
        ));
    }

    artifacts::SkippedPlatform {
        platform: name.to_string(),
        reasons,
        next_steps,
    }
}

fn missing_required_env(platform: &crate::config::PlatformConfig) -> Vec<String> {
    platform
        .required_env
        .iter()
        .filter(|name| {
            platform
                .env
                .get(*name)
                .map(|value| value.trim().is_empty())
                .unwrap_or_else(|| {
                    env::var(name)
                        .map(|value| value.trim().is_empty())
                        .unwrap_or(true)
                })
        })
        .cloned()
        .collect()
}

fn current_host_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
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
    fn build_fails_for_unknown_selected_platform() {
        run(BuildArgs {
            config: temp_config("unknown"),
            platform: Some("windows".to_string()),
            dry_run: true,
        })
        .expect("disabled platform should be skipped");

        let unknown = run(BuildArgs {
            config: temp_config("unknown-name"),
            platform: Some("not-real".to_string()),
            dry_run: true,
        })
        .expect_err("unknown platform should fail");
        assert!(unknown.to_string().contains("unknown release platform"));
    }
}
