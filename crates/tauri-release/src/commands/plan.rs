//! Planning command for checking platform configuration before long Tauri builds start.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("project root: {}", config.project.root.display());
    println!("output:       {}", config.build.out_dir.display());
    println!("cache:        {}", config.build.cache_dir.display());
    for (name, platform) in &config.platforms {
        if platform.enabled {
            println!("platform:     {name}");
            println!("  strategy:  {:?}", platform.strategy);
            println!("  command:   {}", platform.command);
            for artifact in &platform.artifacts {
                println!("  artifact:  {artifact}");
            }
        }
    }
    Ok(())
}
