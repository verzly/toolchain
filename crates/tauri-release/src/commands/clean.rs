//! Clean command for generated release output and configured cache directories.

use crate::cli::CommonArgs;
use crate::config;
use anyhow::Result;
use std::fs;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    for path in [&config.build.out_dir, &config.build.cache_dir] {
        if path.exists() {
            fs::remove_dir_all(path)?;
            println!("removed {}", path.display());
        }
    }
    Ok(())
}
