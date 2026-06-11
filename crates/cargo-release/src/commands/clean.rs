//! Clean command for generated artifacts owned by this tool. Cargo cache cleanup belongs to rust-cache or Cargo itself.

use crate::cli::CommonArgs;
use crate::config;
use anyhow::Result;
use std::fs;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config, args.release_target.as_deref())?;
    let path = &config.build.out_dir;
    if path.exists() {
        fs::remove_dir_all(path)?;
        println!("removed {}", path.display());
    }
    Ok(())
}
