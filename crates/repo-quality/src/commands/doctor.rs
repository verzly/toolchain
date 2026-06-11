use crate::cli::DoctorArgs;
use crate::project::{Language, ProjectProfile};
use crate::shell;
use anyhow::{bail, Result};
use std::path::Path;

pub fn run(args: DoctorArgs) -> Result<()> {
    let profile = ProjectProfile::detect(
        args.root,
        args.config,
        None,
        &[],
        crate::cli::JsRunnerArg::Auto,
    )?;
    let mut failures = Vec::new();
    let mut suggestions = Vec::new();

    if !profile.root.join("hk.pkl").is_file() {
        failures.push("hk.pkl is missing".to_string());
    }
    if !profile.config_path.is_file() {
        suggestions.push(format!(
            "{} is missing; run `repo-quality init` once so future `repo-quality update` \
             runs can reuse the configured workspace and release targets",
            profile.config_path.display()
        ));
    }
    if !shell::command_exists("mise") {
        failures.push("mise is not available on PATH".to_string());
    }
    for command in ["hk", "pkl"] {
        if !tool_available(&profile.root, command) {
            failures.push(format!(
                "{command} is not available on PATH or through `mise exec -- {command}`"
            ));
        }
    }

    if profile.languages.is_empty() {
        failures.push("no supported language profile was detected".to_string());
    }

    if let Some(hooks_path) = git_hooks_path(&profile.root) {
        failures.push(format!(
            "git core.hooksPath is set to `{hooks_path}`; unset it with \
             `git config --local --unset-all core.hooksPath` before running `hk install`"
        ));
    }

    if !profile.has_mise_toml {
        suggestions.push(
            "mise.toml is missing; create it with `repo-quality init` or add hk/pkl manually"
                .to_string(),
        );
    }

    for recommendation in profile.missing_mise_tools() {
        suggestions.push(format!(
            "{} ({})",
            recommendation.command(),
            recommendation.reason
        ));
    }

    if !profile.workspace_root.join(".editorconfig").is_file() {
        suggestions
            .push(".editorconfig is missing in the configured quality workspace".to_string());
    }
    if profile.has_language(&Language::Rust)
        && !profile.workspace_root.join("rustfmt.toml").is_file()
    {
        suggestions.push("rustfmt.toml is missing in the configured quality workspace".to_string());
    }
    if profile.has_language(&Language::Js) {
        if !profile.workspace_root.join(".oxfmtrc.json").is_file() {
            suggestions
                .push(".oxfmtrc.json is missing in the configured quality workspace".to_string());
        }
        if !profile.workspace_root.join(".oxlintrc.json").is_file() {
            suggestions
                .push(".oxlintrc.json is missing in the configured quality workspace".to_string());
        }
    }
    if profile.has_language(&Language::Php) {
        if !profile.workspace_root.join("rector.php").is_file() {
            suggestions
                .push("rector.php is missing in the configured quality workspace".to_string());
        }
        if !profile.has_rector {
            suggestions.push(
                "PHP files were detected; add Rector with `composer require --dev rector/rector`"
                    .to_string(),
            );
        }
        if !profile.has_pest {
            suggestions.push(
                "PHP files were detected; add Pest with `composer require --dev pestphp/pest`"
                    .to_string(),
            );
        }
    }

    if !profile.root.join(".github/workflows/test.yml").is_file() {
        suggestions
            .push(".github/workflows/test.yml is missing; run `repo-quality update`".to_string());
    }

    if profile.release_enabled() {
        for target in &profile.stored_config.release.targets {
            let path = profile
                .root
                .join(format!(".github/workflows/release-{}.yml", target.name));
            if !path.is_file() {
                suggestions.push(format!(
                    "{} is missing; run `repo-quality update`",
                    path.display()
                ));
            }
        }
    }

    if !suggestions.is_empty() {
        println!("Recommendations:");
        for suggestion in &suggestions {
            println!("- {suggestion}");
        }
    }

    if failures.is_empty() {
        println!("Repository quality setup looks ready.");
        println!("Run `mise exec -- hk check` to execute the configured quality gates.");
        Ok(())
    } else {
        for failure in &failures {
            eprintln!("- {failure}");
        }
        bail!("repository quality setup is incomplete")
    }
}

fn git_hooks_path(root: &Path) -> Option<String> {
    let value = shell::output(
        root,
        "git",
        ["config", "--local", "--get", "core.hooksPath"],
    )
    .ok()?;
    let value = value.trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn tool_available(root: &Path, command: &str) -> bool {
    shell::command_exists(command)
        || shell::succeeds(root, "mise", ["exec", "--", command, "--version"])
}
