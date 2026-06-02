//! Clean command for generated artifacts owned by this tool. Cargo cache cleanup belongs to rust-cache or Cargo itself.

use anyhow::Result;
use std::fs;
use crate::cli::CommonArgs;
use crate::config;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let path = &config.build.out_dir;
    if path.exists() {
        fs::remove_dir_all(path)?;
        println!("removed {}", path.display());
    }
    Ok(())
}
