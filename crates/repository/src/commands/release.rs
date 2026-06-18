use crate::cli::{ReleaseArgs, ReleaseCommand, ReleaseRemoveArgs, ReleaseSetArgs};
use crate::project::{ProjectProfile, ReleaseTarget};
use crate::release::{remove_release_target, upsert_release_target, write_profile_config};
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(args: ReleaseArgs) -> Result<()> {
    let command = args.command.unwrap_or(ReleaseCommand::Tui);
    let mut profile = ProjectProfile::detect(
        args.root,
        args.config,
        None,
        &[],
        crate::cli::JsRunnerArg::Auto,
    )?;

    match command {
        ReleaseCommand::List => list_targets(&profile),
        ReleaseCommand::Show(args) => show_target(&profile, &args.target),
        ReleaseCommand::Set(args) => set_target(&mut profile, &args),
        ReleaseCommand::Remove(args) => remove_target(&mut profile, &args),
        ReleaseCommand::Tui => run_tui(&mut profile),
    }
}

fn list_targets(profile: &ProjectProfile) -> Result<()> {
    if profile.stored_config.release.targets.is_empty() {
        println!("No release targets configured.");
        return Ok(());
    }

    println!("Configured release targets:");
    for target in &profile.stored_config.release.targets {
        println!(
            "- {} | path={} | strategy={} | workflow={} | repository={}",
            target.name, target.path, target.strategy, target.workflow, target.repository
        );
    }
    Ok(())
}

fn show_target(profile: &ProjectProfile, name: &str) -> Result<()> {
    let target = profile
        .stored_config
        .release
        .targets
        .iter()
        .find(|target| target.name == name)
        .with_context(|| format!("release target not found: {name}"))?;
    print_target(target);
    Ok(())
}

fn set_target(profile: &mut ProjectProfile, args: &ReleaseSetArgs) -> Result<()> {
    profile.stored_config.release.enabled = true;
    let name = upsert_release_target(profile, args)?;
    write_profile_config(profile)?;
    println!(
        "Saved release target `{name}` to {}",
        profile.config_display()
    );
    Ok(())
}

fn remove_target(profile: &mut ProjectProfile, args: &ReleaseRemoveArgs) -> Result<()> {
    if args.target.is_none() && args.path.is_none() {
        bail!("pass a target name or --path to remove a release target");
    }

    if !args.yes {
        let label = args
            .target
            .clone()
            .or_else(|| args.path.as_ref().map(|path| path.display().to_string()))
            .unwrap_or_else(|| "selected target".into());
        if !confirm(&format!("Remove release target {label}?"))? {
            println!("No changes made.");
            return Ok(());
        }
    }

    let removed = remove_release_target(profile, args.target.as_deref(), args.path.as_deref())?;
    let Some(removed) = removed else {
        println!("No matching release target found.");
        return Ok(());
    };

    write_profile_config(profile)?;
    println!("Removed release target `{}`.", removed.name);
    Ok(())
}

fn run_tui(profile: &mut ProjectProfile) -> Result<()> {
    loop {
        println!();
        println!("Datarose release targets ({})", profile.config_display());
        println!("----------------------------------------");
        if profile.stored_config.release.targets.is_empty() {
            println!("No release targets configured.");
        } else {
            for (index, target) in profile.stored_config.release.targets.iter().enumerate() {
                println!(
                    "{}: {} | {} | {} | {}",
                    index + 1,
                    target.name,
                    target.path,
                    target.strategy,
                    target.workflow
                );
            }
        }
        println!();
        println!("a) add  e) edit  d) delete  v) view  q) quit");
        let choice = prompt("Choice")?;
        match choice.trim() {
            "a" | "add" => interactive_set(profile, None)?,
            "e" | "edit" => {
                if let Some(index) = select_target(profile)? {
                    interactive_set(profile, Some(index))?;
                }
            }
            "d" | "delete" | "remove" => {
                if let Some(index) = select_target(profile)? {
                    let target = profile.stored_config.release.targets[index].name.clone();
                    if confirm(&format!("Remove release target {target}?"))? {
                        profile.stored_config.release.targets.remove(index);
                        write_profile_config(profile)?;
                        println!("Removed release target `{target}`.");
                    }
                }
            }
            "v" | "view" | "show" => {
                if let Some(index) = select_target(profile)? {
                    print_target(&profile.stored_config.release.targets[index]);
                }
            }
            "q" | "quit" | "exit" => return Ok(()),
            _ => println!("Unknown choice."),
        }
    }
}

fn interactive_set(profile: &mut ProjectProfile, index: Option<usize>) -> Result<()> {
    let existing = index.map(|index| profile.stored_config.release.targets[index].clone());
    let path_default = existing
        .as_ref()
        .map(|target| target.path.as_str())
        .unwrap_or("");
    let path = prompt_default("Target path", path_default)?;
    if path.trim().is_empty() {
        println!("Target path is required.");
        return Ok(());
    }

    let name_default = existing
        .as_ref()
        .map(|target| target.name.as_str())
        .unwrap_or("");
    let repository_default = existing
        .as_ref()
        .map(|target| target.repository.as_str())
        .unwrap_or("");
    let strategy_default = existing
        .as_ref()
        .map(|target| target.strategy.as_str())
        .unwrap_or("same-repo");
    let workflow_default = existing
        .as_ref()
        .map(|target| target.workflow.as_str())
        .unwrap_or("custom");

    let name = prompt_default("Name", name_default)?;
    let repository = prompt_default("Publish repository", repository_default)?;
    let strategy = prompt_default("Strategy [same-repo|self-hosted|custom]", strategy_default)?;
    let workflow = prompt_default("Workflow [managed|preserve|custom]", workflow_default)?;

    let args = ReleaseSetArgs {
        name: optional_string(name),
        path: PathBuf::from(path),
        repository: optional_string(repository),
        strategy: parse_strategy(&strategy),
        workflow: parse_workflow(&workflow),
        workspace: existing.as_ref().map(|target| target.workspace.clone()),
        source_kind: existing.as_ref().map(|target| target.source_kind.clone()),
        cargo_package: existing.as_ref().map(|target| target.cargo_package.clone()),
        cargo_binary: existing.as_ref().map(|target| target.cargo_binary.clone()),
        cargo_out_dir: existing.as_ref().map(|target| target.cargo_out_dir.clone()),
        distribution_path: existing
            .as_ref()
            .map(|target| target.distribution_path.clone()),
        version_file: existing.as_ref().map(|target| target.version_file.clone()),
        source_tag_prefix: existing
            .as_ref()
            .map(|target| target.source_tag_prefix.clone()),
        allow_missing_path: false,
    };

    profile.stored_config.release.enabled = true;
    let name = upsert_release_target(profile, &args)?;
    write_profile_config(profile)?;
    println!("Saved release target `{name}`.");
    Ok(())
}

fn select_target(profile: &ProjectProfile) -> Result<Option<usize>> {
    if profile.stored_config.release.targets.is_empty() {
        println!("No release targets configured.");
        return Ok(None);
    }
    let value = prompt("Target number")?;
    let index = value
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1));
    match index {
        Some(index) if index < profile.stored_config.release.targets.len() => Ok(Some(index)),
        _ => {
            println!("Invalid target selection.");
            Ok(None)
        }
    }
}

fn print_target(target: &ReleaseTarget) {
    println!("Release target `{}`", target.name);
    println!("  path: {}", target.path);
    println!("  workspace: {}", empty_as_dash(&target.workspace));
    println!("  strategy: {}", target.strategy);
    println!("  workflow: {}", target.workflow);
    println!("  source kind: {}", empty_as_dash(&target.source_kind));
    println!("  repository: {}", empty_as_dash(&target.repository));
    println!("  action surface: {}", action_surface_for_target(target));
    println!("  cargo package: {}", empty_as_dash(&target.cargo_package));
    println!("  cargo binary: {}", empty_as_dash(&target.cargo_binary));
    println!("  version file: {}", empty_as_dash(&target.version_file));
    println!(
        "  source tag prefix: {}",
        empty_as_dash(&target.source_tag_prefix)
    );
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

fn prompt_default(label: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        prompt(label)
    } else {
        let value = prompt(&format!("{label} [{default}]"))?;
        if value.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(value)
        }
    }
}

fn confirm(label: &str) -> Result<bool> {
    let value = prompt(&format!("{label} [y/N]"))?;
    Ok(matches!(value.as_str(), "y" | "Y" | "yes" | "YES"))
}

fn optional_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn parse_strategy(value: &str) -> Option<crate::cli::ReleaseStrategyArg> {
    match value {
        "same-repo" => Some(crate::cli::ReleaseStrategyArg::SameRepo),
        "distribution-repo" => Some(crate::cli::ReleaseStrategyArg::DistributionRepo),
        "self-hosted" => Some(crate::cli::ReleaseStrategyArg::SelfHosted),
        "custom" => Some(crate::cli::ReleaseStrategyArg::Custom),
        _ => None,
    }
}

fn parse_workflow(value: &str) -> Option<crate::cli::ReleaseWorkflowArg> {
    match value {
        "managed" => Some(crate::cli::ReleaseWorkflowArg::Managed),
        "preserve" => Some(crate::cli::ReleaseWorkflowArg::Preserve),
        "custom" => Some(crate::cli::ReleaseWorkflowArg::Custom),
        _ => None,
    }
}

fn action_surface_for_target(target: &ReleaseTarget) -> &str {
    if target.name == "verzly" {
        "action.yml, actions/"
    } else {
        "-"
    }
}

fn empty_as_dash(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}
