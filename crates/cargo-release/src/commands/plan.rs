//! Planning command for reviewing target configuration before any build process starts.

use anyhow::Result;
use crate::cli::CommonArgs;
use crate::config;

pub fn run(args: CommonArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    println!("project root: {}", config.project.root.display());
    println!("output:       {}", config.build.out_dir.display());
    for (name, target) in &config.targets {
        if target.enabled {
            println!("target:       {name}");
            println!("  triple:    {}", target.triple);
            println!("  strategy:  {:?}", target.strategy);
            println!("  command:   {}", target.command);
            for artifact in &target.artifacts {
                println!("  artifact:  {artifact}");
            }
        }
    }
    Ok(())
}
