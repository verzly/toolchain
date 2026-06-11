//! hk configuration rendering.

use crate::project::{JsRunner, Language, ProjectProfile};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
struct Step {
    name: String,
    check: String,
    fix: Option<String>,
    stage: Vec<String>,
    depends: Vec<String>,
}

pub fn render_hk_config(profile: &ProjectProfile) -> String {
    let mut format_steps = Vec::new();
    let mut quality_steps = Vec::new();

    if profile.has_language(&Language::Rust) {
        format_steps.push(Step {
            name: "format-rust".into(),
            check: "cargo fmt --all -- --check".into(),
            fix: Some("cargo fmt --all".into()),
            stage: vec!["**/*.rs".into()],
            depends: vec![],
        });
        quality_steps.push(Step {
            name: "lint-rust".into(),
            check: "cargo clippy --workspace --all-targets -- -D warnings".into(),
            fix: None,
            stage: vec![],
            depends: vec!["format-rust".into()],
        });
        quality_steps.push(Step {
            name: "test-rust".into(),
            check: "cargo test --workspace --all-targets".into(),
            fix: None,
            stage: vec![],
            depends: vec!["format-rust".into(), "lint-rust".into()],
        });
    }

    if profile.has_language(&Language::Js) {
        add_js_steps(profile, &mut format_steps, &mut quality_steps);
    }

    if profile.has_language(&Language::Php) {
        add_php_steps(profile, &mut format_steps, &mut quality_steps);
    }

    render_pkl(&format_steps, &quality_steps)
}

fn add_js_steps(
    profile: &ProjectProfile,
    format_steps: &mut Vec<Step>,
    quality_steps: &mut Vec<Step>,
) {
    let scripts = &profile.package_scripts;
    let runner = profile.js_runner.as_ref().unwrap_or(&JsRunner::Npm);

    let format_fix = script_command(runner, scripts, &["format:js", "format"]);
    let format_check = script_command(runner, scripts, &["format:js:check", "format:check"]);

    if let (Some(check), Some(fix)) = (format_check, format_fix) {
        format_steps.push(Step {
            name: "format-js".into(),
            check,
            fix: Some(fix),
            stage: vec![
                "package.json".into(),
                "*.js".into(),
                "*.mjs".into(),
                "*.cjs".into(),
                "*.ts".into(),
                "*.tsx".into(),
                "**/*.js".into(),
                "**/*.mjs".into(),
                "**/*.cjs".into(),
                "**/*.ts".into(),
                "**/*.tsx".into(),
                "**/*.vue".into(),
            ],
            depends: vec![],
        });
    }

    if let Some(check) = script_command(runner, scripts, &["lint:js", "lint"]) {
        quality_steps.push(Step {
            name: "lint-js".into(),
            check,
            fix: None,
            stage: vec![],
            depends: format_dependency(format_steps, "format-js"),
        });
    }

    if let Some(check) = script_command(runner, scripts, &["test:js", "test:unit", "test"]) {
        quality_steps.push(Step {
            name: "test-js".into(),
            check,
            fix: None,
            stage: vec![],
            depends: format_dependency(format_steps, "format-js"),
        });
    }
}

fn add_php_steps(
    profile: &ProjectProfile,
    format_steps: &mut Vec<Step>,
    quality_steps: &mut Vec<Step>,
) {
    if profile.has_rector {
        format_steps.push(Step {
            name: "format-php".into(),
            check: "composer exec rector -- --dry-run".into(),
            fix: Some("composer exec rector".into()),
            stage: vec!["composer.json".into(), "**/*.php".into()],
            depends: vec![],
        });
    }

    if profile.has_pest {
        quality_steps.push(Step {
            name: "test-php".into(),
            check: "composer exec pest".into(),
            fix: None,
            stage: vec![],
            depends: format_dependency(format_steps, "format-php"),
        });
    }
}

fn script_command(
    runner: &JsRunner,
    scripts: &BTreeSet<String>,
    candidates: &[&str],
) -> Option<String> {
    candidates
        .iter()
        .find(|script| scripts.contains(**script))
        .map(|script| match runner {
            JsRunner::Aube => format!("aube run {script}"),
            JsRunner::Npm => format!("npm run {script}"),
            JsRunner::Pnpm => format!("pnpm run {script}"),
            JsRunner::Yarn => format!("yarn {script}"),
            JsRunner::Bun => format!("bun run {script}"),
        })
}

fn format_dependency(format_steps: &[Step], name: &str) -> Vec<String> {
    if format_steps.iter().any(|step| step.name == name) {
        vec![name.into()]
    } else {
        vec![]
    }
}

fn render_pkl(format_steps: &[Step], quality_steps: &[Step]) -> String {
    let mut out = String::new();
    let schema = "package://github.com/jdx/hk/releases/download/v1.47.0/hk@1.47.0#/Config.pkl";
    out.push_str(&format!("amends \"{schema}\"\n\n"));
    out.push_str("local defaultShell = new Script {\n");
    out.push_str("  linux = \"sh -o errexit -c\"\n");
    out.push_str("  macos = \"sh -o errexit -c\"\n");
    out.push_str("  windows = \"cmd /d /s /c\"\n");
    out.push_str("  other = \"sh -o errexit -c\"\n");
    out.push_str("}\n\n");
    out.push_str("local formatSteps = new Mapping<String, Step> {\n");
    for step in format_steps {
        render_step(&mut out, step);
    }
    out.push_str("}\n\n");
    out.push_str("local qualitySteps = new Mapping<String, Step> {\n");
    for step in quality_steps {
        render_step(&mut out, step);
    }
    out.push_str("}\n\n");
    out.push_str("local fullQualitySteps = new Mapping<String, Step> {\n");
    for step in format_steps.iter().chain(quality_steps) {
        let step_collection = if format_steps.iter().any(|format| format.name == step.name) {
            "format"
        } else {
            "quality"
        };
        out.push_str(&format!(
            "  [\"{}\"] = {}Steps[\"{}\"]\n",
            step.name, step_collection, step.name
        ));
    }
    out.push_str("}\n\n");
    out.push_str("hooks {\n");
    if !format_steps.is_empty() {
        out.push_str("  [\"pre-commit\"] {\n");
        out.push_str("    fix = true\n");
        out.push_str("    steps = formatSteps\n");
        out.push_str("  }\n\n");
    }
    out.push_str("  [\"pre-push\"] {\n");
    out.push_str("    steps = fullQualitySteps\n");
    out.push_str("  }\n\n");
    if !format_steps.is_empty() {
        out.push_str("  [\"fix\"] {\n");
        out.push_str("    fix = true\n");
        out.push_str("    steps = formatSteps\n");
        out.push_str("  }\n\n");
    }
    out.push_str("  [\"check\"] {\n");
    out.push_str("    steps = fullQualitySteps\n");
    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

fn render_step(out: &mut String, step: &Step) {
    out.push_str(&format!("  [\"{}\"] {{\n", step.name));
    out.push_str("    shell = defaultShell\n");
    if !step.depends.is_empty() {
        out.push_str(&format!("    depends = {}\n", render_list(&step.depends)));
    }
    out.push_str(&format!("    check = \"{}\"\n", escape_pkl(&step.check)));
    if let Some(fix) = &step.fix {
        out.push_str(&format!("    fix = \"{}\"\n", escape_pkl(fix)));
    }
    if !step.stage.is_empty() {
        out.push_str(&format!("    stage = {}\n", render_list(&step.stage)));
    }
    out.push_str("  }\n\n");
}

fn render_list(items: &[String]) -> String {
    let quoted = items
        .iter()
        .map(|item| format!("\"{}\"", escape_pkl(item)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("List({quoted})")
}

fn escape_pkl(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{JsRunner, Language, ProjectProfile};
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn renders_rust_js_php_hooks() {
        let profile = ProjectProfile {
            root: PathBuf::from("."),
            languages: vec![Language::Rust, Language::Js, Language::Php],
            js_runner: Some(JsRunner::Aube),
            package_scripts: BTreeSet::from([
                "format:js".into(),
                "format:js:check".into(),
                "lint:js".into(),
                "test:js".into(),
            ]),
            has_rector: true,
            has_pest: true,
            has_mise_toml: true,
            mise_tools: BTreeSet::from([
                "hk".into(),
                "pkl".into(),
                "rust".into(),
                "aube".into(),
                "php".into(),
            ]),
        };

        let config = render_hk_config(&profile);

        assert!(config.contains("[\"format-rust\"]"));
        assert!(config.contains("aube run test:js"));
        assert!(config.contains("composer exec rector -- --dry-run"));
        assert!(config.contains("[\"pre-push\"]"));
        assert!(config.contains("windows = \"cmd /d /s /c\""));
    }
}
