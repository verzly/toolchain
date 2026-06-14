use crate::cli::{
    CheckArgs, DoctorArgs, InitArgs, JsRunnerArg, PlanArgs, ReleaseArgs, ReleaseCommand, TuiArgs,
    UpdateArgs,
};
use crate::commands;
use crate::project::ProjectProfile;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(args: TuiArgs) -> Result<()> {
    loop {
        let profile = ProjectProfile::detect(
            args.root.clone(),
            args.config.clone(),
            None,
            &[],
            JsRunnerArg::Auto,
        )?;
        print_dashboard(&profile);

        match prompt("Choice")?.trim() {
            "1" | "plan" => run_and_pause(|| {
                commands::plan::run(PlanArgs {
                    root: args.root.clone(),
                    config: args.config.clone(),
                    languages: Vec::new(),
                    js_runner: JsRunnerArg::Auto,
                    workspace: None,
                })
            })?,
            "2" | "init-dry-run" | "init preview" => {
                let workspace = prompt_optional_path("Workspace override")?;
                run_and_pause(|| {
                    commands::init::run(InitArgs {
                        root: args.root.clone(),
                        config: args.config.clone(),
                        force: false,
                        dry_run: true,
                        skip_mise_use: true,
                        skip_hk_install: true,
                        languages: Vec::new(),
                        js_runner: JsRunnerArg::Auto,
                        workspace,
                        skip_style_configs: false,
                        skip_actions: false,
                    })
                })?;
            }
            "3" | "init" | "init apply" => {
                let workspace = prompt_optional_path("Workspace override")?;
                let force = confirm("Overwrite existing managed files")?;
                let run_mise = confirm("Run mise use for missing tools")?;
                let install_hooks = confirm("Run hk install after writing files")?;
                run_and_pause(|| {
                    commands::init::run(InitArgs {
                        root: args.root.clone(),
                        config: args.config.clone(),
                        force,
                        dry_run: false,
                        skip_mise_use: !run_mise,
                        skip_hk_install: !install_hooks,
                        languages: Vec::new(),
                        js_runner: JsRunnerArg::Auto,
                        workspace,
                        skip_style_configs: false,
                        skip_actions: false,
                    })
                })?;
            }
            "4" | "update-dry-run" | "update preview" => run_and_pause(|| {
                commands::init::run_update(UpdateArgs {
                    root: args.root.clone(),
                    config: args.config.clone(),
                    force: false,
                    dry_run: true,
                    skip_mise_use: true,
                    skip_hk_install: true,
                    skip_style_configs: false,
                    skip_actions: false,
                })
            })?,
            "5" | "update" | "update apply" => {
                let force = confirm("Overwrite existing managed files")?;
                let run_mise = confirm("Run mise use for missing tools")?;
                let install_hooks = confirm("Run hk install after writing files")?;
                run_and_pause(|| {
                    commands::init::run_update(UpdateArgs {
                        root: args.root.clone(),
                        config: args.config.clone(),
                        force,
                        dry_run: false,
                        skip_mise_use: !run_mise,
                        skip_hk_install: !install_hooks,
                        skip_style_configs: false,
                        skip_actions: false,
                    })
                })?;
            }
            "6" | "release" | "release targets" => run_and_pause(|| {
                commands::release::run(ReleaseArgs {
                    root: args.root.clone(),
                    config: args.config.clone(),
                    command: Some(ReleaseCommand::Tui),
                })
            })?,
            "7" | "doctor" => run_and_pause(|| {
                commands::doctor::run(DoctorArgs {
                    root: args.root.clone(),
                    config: args.config.clone(),
                })
            })?,
            "8" | "check" => run_and_pause(|| {
                commands::check::run(CheckArgs {
                    root: args.root.clone(),
                    config: args.config.clone(),
                })
            })?,
            "q" | "quit" | "exit" => return Ok(()),
            _ => {
                println!("Unknown choice.");
                pause()?;
            }
        }
    }
}

fn print_dashboard(profile: &ProjectProfile) {
    let languages = profile
        .languages
        .iter()
        .map(|language| language.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let js_runner = profile
        .js_runner
        .as_ref()
        .map(|runner| runner.as_str())
        .unwrap_or("-");

    println!();
    println!("Repository TUI");
    println!("==============");
    println!("Root: {}", profile.root.display());
    println!("Config: {}", profile.config_display());
    println!("Workspace: {}", profile.workspace_display());
    println!(
        "Languages: {}",
        if languages.is_empty() {
            "-"
        } else {
            languages.as_str()
        }
    );
    println!("JavaScript runner: {js_runner}");
    println!(
        "Release targets: {}",
        profile.stored_config.release.targets.len()
    );
    println!();
    println!("1) plan");
    println!("2) init dry-run");
    println!("3) init apply");
    println!("4) update dry-run");
    println!("5) update apply");
    println!("6) release targets");
    println!("7) doctor");
    println!("8) check");
    println!("q) quit");
}

fn run_and_pause(action: impl FnOnce() -> Result<()>) -> Result<()> {
    if let Err(error) = action() {
        eprintln!("Error: {error:#}");
    }
    pause()
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush().context("failed to flush stdout")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .context("failed to read stdin")?;
    Ok(value.trim().to_string())
}

fn prompt_optional_path(label: &str) -> Result<Option<PathBuf>> {
    let value = prompt(&format!("{label} [use detected]"))?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

fn confirm(label: &str) -> Result<bool> {
    let value = prompt(&format!("{label} [y/N]"))?;
    Ok(matches!(value.as_str(), "y" | "Y" | "yes" | "YES"))
}

fn pause() -> Result<()> {
    let _ = prompt("Press Enter to continue")?;
    Ok(())
}
