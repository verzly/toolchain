//! Interactive release planning and build cockpit.

use crate::cli::{JsRunnerArg, TuiArgs};
use crate::commands::interactive::prompt_default;
use crate::output::{
    command as command_style, command_cell, compact_table, empty_as_dash, header_cell, key_cell,
    note, print_release_target_table, section, style_compact_columns, value_cell, warning,
    warning_cell,
};
use crate::project::{ProjectProfile, ReleaseTarget};
use crate::shell;
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseFlowMode {
    Plan,
    Act,
}

pub fn run(args: &TuiArgs, mode: ReleaseFlowMode) -> Result<()> {
    let profile = ProjectProfile::detect(
        args.root.clone(),
        args.config.clone(),
        None,
        &[],
        JsRunnerArg::Auto,
    )?;
    if profile.stored_config.release.targets.is_empty() {
        warning("No release targets configured. Use /targets first.");
        return Ok(());
    }

    section("Release cockpit");
    print_release_target_table(&profile);
    println!();
    let target_selector = prompt_default("Target [all|name]", "all")?;
    let version = prompt_default("Version [for example 1.2.3]", "")?;
    if version.trim().is_empty() {
        warning("No version provided. No release command was run.");
        return Ok(());
    }
    let targets = select_release_targets(&profile, &target_selector)?;

    println!();
    print_release_commands(&profile, &targets, &version);

    if mode == ReleaseFlowMode::Plan {
        note("PLAN mode only prints commands. Switch to ACT to run a release step.");
        return Ok(());
    }

    println!();
    note("Runnable ACT steps:");
    println!("- {}", command_style("commands"));
    println!("- {}", command_style("github-plan"));
    println!("- {}", command_style("build-dry-run"));
    println!("- {}", command_style("build"));
    let operation = prompt_default(
        "Operation [commands|github-plan|build-dry-run|build]",
        "commands",
    )?;
    match operation.as_str() {
        "commands" => Ok(()),
        "github-plan" => {
            for target in targets {
                run_release_tool(
                    &profile,
                    "github-release",
                    vec![
                        "plan".into(),
                        "--config".into(),
                        profile.config_display(),
                        "--release-target".into(),
                        target.name.clone(),
                        "--version".into(),
                        version.clone(),
                    ],
                )?;
            }
            Ok(())
        }
        "build-dry-run" | "build" => {
            let dry_run = operation == "build-dry-run";
            for target in targets {
                if target.cargo_binary.trim().is_empty() {
                    warning(format!(
                        "Skipping `{}`: no cargo_binary configured for this target.",
                        target.name
                    ));
                    continue;
                }
                let mut tool_args = vec![
                    "build".into(),
                    "--config".into(),
                    profile.config_display(),
                    "--release-target".into(),
                    target.name.clone(),
                    "--version".into(),
                    version.clone(),
                ];
                if dry_run {
                    tool_args.push("--dry-run".into());
                }
                run_release_tool(&profile, "cargo-release", tool_args)?;
            }
            Ok(())
        }
        _ => {
            warning(format!("Unknown release operation `{operation}`."));
            Ok(())
        }
    }
}

fn select_release_targets<'a>(
    profile: &'a ProjectProfile,
    selector: &str,
) -> Result<Vec<&'a ReleaseTarget>> {
    let selector = selector.trim();
    if selector.is_empty() || selector == "all" {
        return Ok(profile.stored_config.release.targets.iter().collect());
    }

    let Some(target) = profile
        .stored_config
        .release
        .targets
        .iter()
        .find(|target| target.name == selector)
    else {
        bail!("release target not found: {selector}");
    };

    Ok(vec![target])
}

fn print_release_commands(profile: &ProjectProfile, targets: &[&ReleaseTarget], version: &str) {
    note(format!("Release commands for {version}:"));
    if targets.len() == profile.stored_config.release.targets.len() {
        println!(
            "- {}",
            command_style(format!(
                "gh workflow run release-all.yml -f version={version} -f prerelease=auto"
            ))
        );
    }

    for target in targets {
        println!();
        let mut target_table = compact_table();
        target_table.set_header([header_cell("Field"), header_cell("Value")]);
        style_compact_columns(&mut target_table);
        target_table.add_row([key_cell("target"), value_cell(&target.name, 42)]);
        target_table.add_row([
            key_cell("source"),
            value_cell(empty_as_dash(&target.path), 42),
        ]);
        target_table.add_row([
            key_cell("source tag"),
            value_cell(
                source_tag_for_version(&target.source_tag_prefix, version),
                42,
            ),
        ]);
        target_table.add_row([
            key_cell("public repo"),
            value_cell(empty_as_dash(&target.repository), 42),
        ]);
        target_table.add_row([
            key_cell("public tag"),
            value_cell(format!("v{version}"), 42),
        ]);
        target_table.add_row([
            key_cell("action surface"),
            value_cell(action_surface_for_target(target), 42),
        ]);
        target_table.add_row([
            key_cell("executable"),
            value_cell(empty_as_dash(&target.cargo_binary), 42),
        ]);
        println!("{target_table}");

        let mut command_table = compact_table();
        command_table.set_header([header_cell("Step"), header_cell("Command")]);
        style_compact_columns(&mut command_table);
        command_table.add_row([
            key_cell("workflow"),
            command_cell(
                format!(
                    "gh workflow run release-{}.yml -f version={version} -f prerelease=auto",
                    target.name
                ),
                112,
            ),
        ]);
        command_table.add_row([
            key_cell("plan"),
            command_cell(
                format!(
                    "github-release plan --config {} --release-target {} --version {version}",
                    profile.config_display(),
                    target.name
                ),
                112,
            ),
        ]);
        if !target.cargo_binary.trim().is_empty() {
            command_table.add_row([
                key_cell("build preview"),
                warning_cell(
                    format!(
                        "cargo-release build --config {} --release-target {} --version {version} --dry-run",
                        profile.config_display(),
                        target.name
                    ),
                    112,
                ),
            ]);
            command_table.add_row([
                key_cell("build"),
                command_cell(
                    format!(
                        "cargo-release build --config {} --release-target {} --version {version}",
                        profile.config_display(),
                        target.name
                    ),
                    112,
                ),
            ]);
        }
        command_table.add_row([
            key_cell("publish"),
            command_cell(
                format!(
                    "github-release publish --config {} --release-target {} --version {version} --assets dist/release",
                    profile.config_display(),
                    target.name
                ),
                112,
            ),
        ]);
        println!("{command_table}");
    }
}

fn run_release_tool(profile: &ProjectProfile, tool: &str, args: Vec<String>) -> Result<()> {
    if shell::command_exists(tool) {
        return shell::run(&profile.root, tool, args);
    }

    if profile.root.join("Cargo.toml").is_file() {
        let mut cargo_args = vec![
            "run".to_string(),
            "-p".to_string(),
            tool.to_string(),
            "--".into(),
        ];
        cargo_args.extend(args);
        return shell::run(&profile.root, "cargo", cargo_args);
    }

    bail!("`{tool}` is not available on PATH. Install it or run the printed CLI command manually.")
}

fn source_tag_for_version(prefix: &str, version: &str) -> String {
    if prefix.trim().is_empty() {
        format!("v{version}")
    } else {
        format!("{prefix}{version}")
    }
}

fn action_surface_for_target(target: &crate::project::ReleaseTarget) -> &str {
    if target.name == "verzly" {
        "action.yml, actions/"
    } else {
        "-"
    }
}
