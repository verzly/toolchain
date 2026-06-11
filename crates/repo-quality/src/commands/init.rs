use crate::cli::InitArgs;
use crate::project::ProjectProfile;
use crate::quality::render_hk_config;
use crate::shell;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn run(args: InitArgs) -> Result<()> {
    let profile = ProjectProfile::detect(args.root, &args.languages, args.js_runner)?;
    if profile.languages.is_empty() {
        bail!(
            "no supported language profile detected; pass --language rust, \
             --language js, or --language php"
        );
    }

    let config = render_hk_config(&profile);
    let hk_path = profile.root.join("hk.pkl");

    if args.dry_run {
        print_plan(&profile, &config, args.skip_mise_use, args.skip_hk_install);
        return Ok(());
    }

    if hk_path.exists() && !args.force {
        bail!(
            "{} already exists; pass --force to overwrite it",
            hk_path.display()
        );
    }

    if !args.skip_mise_use {
        for recommendation in profile.missing_mise_tools() {
            let spec = format!("{}@{}", recommendation.tool, recommendation.version);
            shell::run(&profile.root, "mise", ["use", spec.as_str()]).with_context(|| {
                format!("failed to install {} through mise", recommendation.tool)
            })?;
        }
    }

    fs::write(&hk_path, config)
        .with_context(|| format!("failed to write {}", hk_path.display()))?;
    println!("Wrote {}", hk_path.display());

    if !args.skip_hk_install {
        install_hk_hooks(&profile.root)?;
    }

    println!("Repository quality hooks are ready.");
    println!("Run `hk check` before pushing or let the pre-push hook run automatically.");
    Ok(())
}

fn print_plan(profile: &ProjectProfile, config: &str, skip_mise: bool, skip_hk: bool) {
    println!("Repository: {}", profile.root.display());
    println!("Languages: {:?}", profile.languages);
    if !skip_mise {
        for recommendation in profile.missing_mise_tools() {
            println!("Would run: {}", recommendation.command());
        }
    }
    println!("Would write: {}", profile.root.join("hk.pkl").display());
    if !skip_hk {
        println!("Would run: mise exec -- hk install");
    }
    println!("\n{config}");
}

fn install_hk_hooks(root: &Path) -> Result<()> {
    if shell::run(root, "mise", ["exec", "--", "hk", "install"]).is_ok() {
        return Ok(());
    }

    shell::run(root, "hk", ["install"]).context("failed to install hk git hooks")
}
