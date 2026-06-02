//! Publishes a GitHub Release without touching local branches.
//!
//! This command is designed for distribution repositories whose source code lives
//! elsewhere. In the Verzly toolchain model, the source monorepo is merged and
//! tagged first, then this command publishes `vX.Y.Z` to the public repository
//! using release notes generated from the source tag.

use anyhow::Result;

use crate::cli::PublishArgs;
use crate::config;
use crate::domain;
use crate::github;
use crate::output;

pub fn run(args: PublishArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = domain::build_plan(
        &config,
        &args.version,
        None,
        None,
        Some(args.prerelease),
    )?;

    output::print_plan(&plan);
    github::create_release(&plan, args.assets.as_deref(), args.dry_run)?;
    output::write_github_outputs(&plan)?;

    Ok(())
}
