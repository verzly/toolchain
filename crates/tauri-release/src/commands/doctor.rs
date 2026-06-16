//! Environment checks. These should make platform limitations visible before contributors start debugging builds.

use crate::cli::CommonArgs;
use crate::config::{self, Strategy};
use crate::process;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("cargo: {}", available_text(process::available("cargo")));
    println!(
        "tauri: {}",
        available_text(process::available("cargo-tauri"))
    );
    println!(
        "{}: {}",
        config.build.container_engine.executable(),
        available_text(process::available(
            config.build.container_engine.executable()
        ))
    );
    for (name, platform) in &config.platforms {
        if platform.enabled && platform.strategy == Strategy::Container && platform.image.is_none()
        {
            println!("platform {name}: missing container image");
        }
        if let Some(host_os) = platform.required_host_os.as_deref() {
            println!("platform {name} host requirement: {host_os}");
        }
        for command in &platform.required_commands {
            println!(
                "platform {name} command {command}: {}",
                available_text(process::available(command))
            );
        }
        for env_name in &platform.required_env {
            let present = platform
                .env
                .get(env_name)
                .map(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    std::env::var(env_name)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                });
            println!(
                "platform {name} env {env_name}: {}",
                available_text(present)
            );
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
