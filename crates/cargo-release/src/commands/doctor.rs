//! Environment checks for contributors and CI. Failures here should explain missing local tooling clearly.

use crate::cli::CommonArgs;
use crate::config::{self, Strategy};
use crate::process;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config, args.release_target.as_deref())?;
    println!("cargo: {}", available_text(process::available("cargo")));
    println!(
        "{}: {}",
        config.build.container_engine.executable(),
        available_text(process::available(
            config.build.container_engine.executable()
        ))
    );
    for (name, target) in &config.targets {
        if target.enabled && target.strategy == Strategy::Container && target.image.is_none() {
            println!("target {name}: missing container image");
        }
        for env_name in &target.required_env {
            let present = target
                .env
                .get(env_name)
                .map(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    std::env::var(env_name)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                });
            println!("target {name} env {env_name}: {}", available_text(present));
        }
    }
    Ok(())
}

fn available_text(value: bool) -> &'static str {
    if value {
        "ok"
    } else {
        "missing"
    }
}
