//! Dry-run planning command. It is used by contributors to understand the release without changing the repository.

use crate::cli::PlanArgs;
use crate::config;
use crate::domain;
use crate::output;
use crate::version_files;
use anyhow::Result;

pub fn run(args: PlanArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = domain::build_plan(
        &config,
        &args.version,
        args.target_branch.as_deref(),
        args.release_branch.as_deref(),
        None,
    )?;

    output::print_plan(&plan);
    for file in &config.files {
        println!(
            "file:           {} ({:?}) -> {}",
            file.path.display(),
            file.kind,
            version_files::render_value(file, &plan)
        );
    }

    Ok(())
}
