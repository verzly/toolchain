use crate::cli::{InitArgs, UpdateArgs};
use crate::commands::check::collect_config_issues;
use crate::project::{render_datarose_config, ProjectProfile};
use crate::quality::render_hk_config;
use crate::shell;
use crate::standards::{self, ManagedFile, WriteOutcome};
use crate::workflow::{release_workflow_files, render_test_workflow};
use anyhow::{bail, Context, Result};
use std::path::Path;

pub fn run(args: InitArgs) -> Result<()> {
    let options = ApplyOptions {
        root: args.root,
        config: args.config,
        workspace: args.workspace,
        languages: args.languages,
        js_runner: args.js_runner,
        force: args.force,
        dry_run: args.dry_run,
        skip_mise_use: args.skip_mise_use,
        skip_hk_install: args.skip_hk_install,
        skip_style_configs: args.skip_style_configs,
        skip_actions: args.skip_actions,
        from_stored_config: false,
    };
    apply(options)
}

pub fn run_update(args: UpdateArgs) -> Result<()> {
    let options = ApplyOptions {
        root: args.root,
        config: args.config,
        workspace: None,
        languages: Vec::new(),
        js_runner: crate::cli::JsRunnerArg::Auto,
        force: args.force,
        dry_run: args.dry_run,
        skip_mise_use: args.skip_mise_use,
        skip_hk_install: args.skip_hk_install,
        skip_style_configs: args.skip_style_configs,
        skip_actions: args.skip_actions,
        from_stored_config: true,
    };
    apply(options)
}

struct ApplyOptions {
    root: std::path::PathBuf,
    config: Option<std::path::PathBuf>,
    workspace: Option<std::path::PathBuf>,
    languages: Vec<crate::cli::LanguageArg>,
    js_runner: crate::cli::JsRunnerArg,
    force: bool,
    dry_run: bool,
    skip_mise_use: bool,
    skip_hk_install: bool,
    skip_style_configs: bool,
    skip_actions: bool,
    from_stored_config: bool,
}

fn apply(options: ApplyOptions) -> Result<()> {
    let profile = ProjectProfile::detect(
        options.root,
        options.config,
        options.workspace,
        &options.languages,
        options.js_runner,
    )?;
    if profile.languages.is_empty() {
        bail!(
            "no supported language profile detected; pass --language rust, \
             --language js, or --language php"
        );
    }

    if options.from_stored_config {
        let issues = collect_config_issues(&profile)?;
        if !issues.is_empty() {
            eprintln!("datarose configuration warnings:");
            for issue in &issues {
                eprintln!("- {issue}");
            }
            eprintln!("Run `repository check` to fail on these issues in CI or pre-push.");
        }
    }

    let hk_config = render_hk_config(&profile);
    let repo_config = render_datarose_config(&profile);
    let hk_path = profile.root.join("hk.pkl");
    let repo_config_path = profile.config_path.clone();
    let actions_path = profile.root.join(".github/workflows/test.yml");

    let mut managed_files = vec![
        ManagedFile {
            path: hk_path.clone(),
            content: hk_config,
            force: options.force || options.from_stored_config,
        },
        ManagedFile {
            path: repo_config_path.clone(),
            content: repo_config,
            force: true,
        },
    ];

    if !options.skip_style_configs {
        managed_files.extend(standards::style_files(&profile, options.force));
    }
    if !options.skip_actions {
        managed_files.push(ManagedFile {
            path: actions_path,
            content: render_test_workflow(&profile),
            force: options.force || options.from_stored_config,
        });
        managed_files.extend(release_workflow_files(
            &profile,
            options.force || options.from_stored_config,
        ));
    }

    if options.dry_run {
        print_plan(
            &profile,
            &managed_files,
            options.skip_mise_use,
            options.skip_hk_install,
        );
        return Ok(());
    }

    if options.from_stored_config && !repo_config_path.exists() {
        bail!(
            "{} is missing; run `repository init` once before `repository update`",
            repo_config_path.display()
        );
    }

    if !options.from_stored_config && hk_path.exists() && !options.force {
        bail!(
            "{} already exists; pass --force to overwrite it",
            hk_path.display()
        );
    }

    if !options.skip_mise_use {
        for recommendation in profile.missing_mise_tools() {
            let spec = format!("{}@{}", recommendation.tool, recommendation.version);
            shell::run(&profile.root, "mise", ["use", spec.as_str()]).with_context(|| {
                format!("failed to install {} through mise", recommendation.tool)
            })?;
        }
    }

    for outcome in standards::write_files(&managed_files)? {
        match outcome {
            WriteOutcome::Wrote(path) => println!("Wrote {}", path.display()),
            WriteOutcome::Kept(path) => println!("Kept custom {}", path.display()),
        }
    }

    if !options.skip_hk_install {
        install_hk_hooks(&profile.root)?;
    }

    println!("Repository quality hooks are ready.");
    println!("Run `hk check` before pushing or let the pre-push hook run automatically.");
    Ok(())
}

fn print_plan(profile: &ProjectProfile, files: &[ManagedFile], skip_mise: bool, skip_hk: bool) {
    println!("Repository: {}", profile.root.display());
    println!("Workspace: {}", profile.workspace_display());
    println!("Languages: {:?}", profile.languages);
    if !skip_mise {
        for recommendation in profile.missing_mise_tools() {
            println!("Would run: {}", recommendation.command());
        }
    }
    for file in files {
        println!("Would write: {}", file.path.display());
    }
    if !skip_hk {
        println!("Would run: mise exec -- hk install");
    }
}

fn install_hk_hooks(root: &Path) -> Result<()> {
    if shell::run(root, "mise", ["exec", "--", "hk", "install"]).is_ok() {
        return Ok(());
    }

    shell::run(root, "hk", ["install"]).context("failed to install hk git hooks")
}
