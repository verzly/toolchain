//! Planning command for checking platform configuration before long Tauri builds start.

use crate::cli::CommonArgs;
use crate::config;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("project root: {}", config.project.root.display());
    println!("output:       {}", config.build.out_dir.display());
    println!("cache:        {}", config.build.cache_dir.display());
    for (name, platform) in &config.platforms {
        if platform.enabled {
            println!("platform:     {name}");
            println!("  strategy:  {:?}", platform.strategy);
            if let Some(host_os) = platform.required_host_os.as_deref() {
                println!("  host os:   {host_os}");
            }
            if !platform.required_commands.is_empty() {
                println!("  commands:  {}", platform.required_commands.join(", "));
            }
            if !platform.required_env.is_empty() {
                println!("  env:       {}", platform.required_env.join(", "));
            }
            println!("  command:   {}", platform.command);
            for artifact in &platform.artifacts {
                println!("  artifact:  {artifact}");
            }
        }
    }
    Ok(())
}
