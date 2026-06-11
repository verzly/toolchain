use crate::cli::DoctorArgs;
use crate::project::{Language, ProjectProfile};
use crate::shell;
use anyhow::{bail, Result};

pub fn run(args: DoctorArgs) -> Result<()> {
    let profile = ProjectProfile::detect(args.root, &[], crate::cli::JsRunnerArg::Auto)?;
    let mut failures = Vec::new();
    let mut suggestions = Vec::new();

    if !profile.root.join("hk.pkl").is_file() {
        failures.push("hk.pkl is missing".to_string());
    }
    for command in ["mise", "hk", "pkl"] {
        if !shell::command_exists(command) {
            failures.push(format!("{command} is not available on PATH"));
        }
    }

    if profile.languages.is_empty() {
        failures.push("no supported language profile was detected".to_string());
    }

    if !profile.has_mise_toml {
        suggestions.push(
            "mise.toml is missing; create it with `mise use hk@latest pkl@latest` or run `repo-quality init`"
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

    if profile.has_language(&Language::Js) {
        push_missing_script(&mut suggestions, &profile, "format:js", "oxfmt");
        push_missing_script(&mut suggestions, &profile, "format:js:check", "oxfmt --check");
        push_missing_script(&mut suggestions, &profile, "lint:js", "oxlint");
        push_missing_script(&mut suggestions, &profile, "test:js", "vitest");
    }

    if profile.has_language(&Language::Php) && !profile.has_rector {
        suggestions.push(
            "PHP files were detected; add Rector with `composer require --dev rector/rector`"
                .to_string(),
        );
    }
    if profile.has_language(&Language::Php) && !profile.has_pest {
        suggestions.push(
            "PHP files were detected; add Pest with `composer require --dev pestphp/pest`"
                .to_string(),
        );
    }

    if !suggestions.is_empty() {
        println!("Recommendations:");
        for suggestion in &suggestions {
            println!("- {suggestion}");
        }
    }

    if failures.is_empty() {
        println!("Repository quality setup looks ready.");
        println!("Run `hk check` to execute the configured quality gates.");
        Ok(())
    } else {
        for failure in &failures {
            eprintln!("- {failure}");
        }
        bail!("repository quality setup is incomplete")
    }
}

fn push_missing_script(
    suggestions: &mut Vec<String>,
    profile: &ProjectProfile,
    script: &str,
    command: &str,
) {
    if !profile.package_scripts.contains(script) {
        suggestions.push(format!(
            "JavaScript/TypeScript files were detected; add package script `{script}` using `{command}`"
        ));
    }
}
