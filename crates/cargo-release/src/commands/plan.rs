//! Planning command for reviewing target configuration before any build process starts.

use crate::cli::CommonArgs;
use crate::config;
use anyhow::Result;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config, args.release_target.as_deref())?;
    println!("project root: {}", config.project.root.display());
    println!("output:       {}", config.build.out_dir.display());
    for (name, target) in &config.targets {
        if target.enabled {
            println!("target:       {name}");
            println!("  triple:    {}", target.triple);
            println!("  strategy:  {:?}", target.strategy);
            println!("  command:   {}", target.command);
            if !target.required_env.is_empty() {
                println!("  required env: {}", target.required_env.join(", "));
            }
            for artifact in &target.artifacts {
                println!("  artifact:  {artifact}");
            }
        }
    }
    Ok(())
}
