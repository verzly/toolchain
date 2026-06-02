//! Environment checks for contributors and CI. Failures here should explain missing local tooling clearly.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config::{self, Strategy};
use crate::process;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("cargo: {}", available_text(process::available("cargo")));
    println!("{}: {}", config.build.container_engine.executable(), available_text(process::available(config.build.container_engine.executable())));
    for (name, target) in &config.targets {
        if target.enabled && target.strategy == Strategy::Container && target.image.is_none() {
            println!("target {name}: missing container image");
        }
    }
    Ok(())
}

fn available_text(value: bool) -> &'static str {
    if value { "ok" } else { "missing" }
}
