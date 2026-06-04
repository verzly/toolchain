//! Publishes a GitHub Release without touching local branches.
//!
//! This command is designed for distribution repositories whose source code lives
//! elsewhere. In the Verzly toolchain model, the source monorepo is merged and
//! tagged first, then this command publishes `vX.Y.Z` to the public repository
//! using release notes generated from the source tag.

use crate::cli::PublishArgs;
use crate::config;
use crate::domain;
use crate::github;
use crate::output;
use anyhow::Result;

pub fn run(args: PublishArgs) -> Result<()> {
    if args.notes.is_some() && args.notes_file.is_some() {
        anyhow::bail!("use either --notes or --notes-file, not both");
    }

    let config = config::load(&args.config)?;
    let plan = domain::build_plan(&config, &args.version, None, None, Some(args.prerelease))?;

    output::print_plan(&plan);
    github::create_release(
        &plan,
        args.assets.as_deref(),
        github::ReleaseNotesInput {
            body: args.notes.as_deref(),
            file: args.notes_file.as_deref(),
        },
        args.dry_run,
    )?;
    if config.release.floating_tags || args.update_floating_tags {
        github::refresh_floating_tags_for_plan(&plan, args.dry_run)?;
    }
    output::write_github_outputs(&plan)?;

    Ok(())
}
