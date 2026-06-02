//! Environment checks. These should make platform limitations visible before contributors start debugging builds.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config::{self, Strategy};
use crate::process;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("cargo: {}", available_text(process::available("cargo")));
    println!("tauri: {}", available_text(process::available("cargo-tauri")));
    println!("{}: {}", config.build.container_engine.executable(), available_text(process::available(config.build.container_engine.executable())));
    for (name, platform) in &config.platforms {
        if platform.enabled && platform.strategy == Strategy::Container && platform.image.is_none() {
            println!("platform {name}: missing container image");
        }
    }
    Ok(())
}

fn available_text(value: bool) -> &'static str {
    if value { "ok" } else { "missing" }
}
