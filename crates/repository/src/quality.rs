//! hk configuration rendering.

use crate::project::{Language, ProjectProfile};

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
    let repository_check = if profile.root.join("crates/repository/Cargo.toml").is_file() {
        format!(
            "cargo run -p repository -- check --config {}",
            shell_quote(&profile.config_display())
        )
    } else {
        format!(
            "repository check --config {}",
            shell_quote(&profile.config_display())
        )
    };
    let mut quality_steps = vec![Step {
        name: "check-datarose".into(),
        check: repository_check,
        fix: None,
        stage: vec![],
        depends: vec![],
    }];

    if profile.has_language(&Language::Rust) {
        let rustfmt_config = shell_quote(&profile.quality_config_display("rustfmt.toml"));
        let clippy_config_dir = shell_quote(&profile.quality_config_directory_display());
        format_steps.push(Step {
            name: "format-rust".into(),
            check: profile.command(&format!(
                "cargo fmt --all -- --config-path {rustfmt_config} --check"
            )),
            fix: Some(profile.command(&format!(
                "cargo fmt --all -- --config-path {rustfmt_config}"
            ))),
            stage: vec![profile.glob("**/*.rs")],
            depends: vec![],
        });
        quality_steps.push(Step {
            name: "lint-rust".into(),
            check: profile.command(&format!("CLIPPY_CONF_DIR={clippy_config_dir} cargo clippy --workspace --all-targets -- -D warnings")),
            fix: None,
            stage: vec![],
            depends: vec!["format-rust".into(), "check-datarose".into()],
        });
        quality_steps.push(Step {
            name: "test-rust".into(),
            check: profile.command("cargo nextest run --workspace && cargo test --workspace --doc"),
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

trait ProfileCommandExt {
    fn command(&self, command: &str) -> String;
    fn glob(&self, pattern: &str) -> String;
}

impl ProfileCommandExt for ProjectProfile {
    fn command(&self, command: &str) -> String {
        if self.workspace_is_root() {
            command.into()
        } else {
            format!("cd {} && {command}", shell_quote(&self.workspace_display()))
        }
    }

    fn glob(&self, pattern: &str) -> String {
        if self.workspace_is_root() {
            pattern.into()
        } else {
            format!("{}/{}", self.workspace_display(), pattern)
        }
    }
}

fn add_js_steps(
    profile: &ProjectProfile,
    format_steps: &mut Vec<Step>,
    quality_steps: &mut Vec<Step>,
) {
    let oxfmt_config = shell_quote(&profile.quality_config_display(".oxfmtrc.json"));
    let oxlint_config = shell_quote(&profile.quality_config_display(".oxlintrc.json"));
    let vitest_config = shell_quote(&profile.quality_config_display("vitest.config.ts"));

    format_steps.push(Step {
        name: "format-js".into(),
        check: profile.command(&format!("oxfmt --config {oxfmt_config} --check .")),
        fix: Some(profile.command(&format!("oxfmt --config {oxfmt_config} ."))),
        stage: vec![
            profile.glob("package.json"),
            profile.glob("*.js"),
            profile.glob("*.mjs"),
            profile.glob("*.cjs"),
            profile.glob("*.ts"),
            profile.glob("*.tsx"),
            profile.glob("**/*.js"),
            profile.glob("**/*.mjs"),
            profile.glob("**/*.cjs"),
            profile.glob("**/*.ts"),
            profile.glob("**/*.tsx"),
            profile.glob("**/*.vue"),
            profile.glob("**/*.json"),
            profile.glob("**/*.yaml"),
            profile.glob("**/*.yml"),
            profile.glob("**/*.md"),
        ],
        depends: vec![],
    });

    quality_steps.push(Step {
        name: "lint-js".into(),
        check: profile.command(&format!("oxlint --config {oxlint_config} .")),
        fix: None,
        stage: vec![],
        depends: vec!["format-js".into(), "check-datarose".into()],
    });

    quality_steps.push(Step {
        name: "test-js".into(),
        check: profile.command(&format!("vitest run --config {vitest_config}")),
        fix: None,
        stage: vec![],
        depends: vec!["format-js".into(), "lint-js".into()],
    });
}

fn add_php_steps(
    profile: &ProjectProfile,
    format_steps: &mut Vec<Step>,
    quality_steps: &mut Vec<Step>,
) {
    let rector_config = shell_quote(&profile.quality_config_display("rector.php"));
    let pest_config = shell_quote(&profile.quality_config_display("phpunit.xml.dist"));

    format_steps.push(Step {
        name: "format-php".into(),
        check: profile.command(&format!(
            "composer exec rector -- --config {rector_config} --dry-run"
        )),
        fix: Some(profile.command(&format!("composer exec rector -- --config {rector_config}"))),
        stage: vec![profile.glob("composer.json"), profile.glob("**/*.php")],
        depends: vec![],
    });

    quality_steps.push(Step {
        name: "test-php".into(),
        check: profile.command(&format!(
            "composer exec pest -- --configuration {pest_config}"
        )),
        fix: None,
        stage: vec![],
        depends: vec!["format-php".into(), "check-datarose".into()],
    });
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
    render_steps(&mut out, format_steps);
    out.push_str("}\n\n");
    out.push_str("local qualitySteps = new Mapping<String, Step> {\n");
    render_steps(&mut out, quality_steps);
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

fn render_steps(out: &mut String, steps: &[Step]) {
    for (index, step) in steps.iter().enumerate() {
        render_step(out, step);
        if index + 1 < steps.len() {
            out.push('\n');
        }
    }
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
    out.push_str("  }\n");
}

fn render_list(items: &[String]) -> String {
    let quoted = items
        .iter()
        .map(|item| format!("\"{}\"", escape_pkl(item)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("List({quoted})")
}

fn shell_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn escape_pkl(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{DataroseConfig, JsRunner, Language, ProjectProfile};
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn renders_rust_js_php_hooks_without_package_scripts() {
        let profile = ProjectProfile {
            root: PathBuf::from("."),
            workspace: PathBuf::from("workspace/app"),
            workspace_root: PathBuf::from("workspace/app"),
            config_path: PathBuf::from("datarose.toml"),
            languages: vec![Language::Rust, Language::Js, Language::Php],
            js_runner: Some(JsRunner::Aube),
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
            stored_config: DataroseConfig::default(),
        };

        let config = render_hk_config(&profile);

        assert!(config.contains("[\"format-rust\"]"));
        assert!(config.contains(
            "cd \\\"workspace/app\\\" && oxfmt --config \\\"config/.oxfmtrc.json\\\" --check ."
        ));
        assert!(config.contains(
            "cd \\\"workspace/app\\\" && vitest run --config \\\"config/vitest.config.ts\\\""
        ));
        assert!(
            config.contains("composer exec rector -- --config \\\"config/rector.php\\\" --dry-run")
        );
        assert!(config.contains("[\"pre-push\"]"));
        assert!(config.contains("windows = \"cmd /d /s /c\""));
        assert!(!config.contains("  }\n\n}\n\nlocal qualitySteps"));
        assert!(!config.contains("  }\n\n}\n\nlocal fullQualitySteps"));

        insta::assert_snapshot!(step_block(&config, "[\"test-rust\"]"), @r###"["test-rust"] {
    shell = defaultShell
    depends = List("format-rust", "lint-rust")
    check = "cd \"workspace/app\" && cargo nextest run --workspace && cargo test --workspace --doc"
  }"###);
    }

    fn step_block(config: &str, marker: &str) -> String {
        let start = config.find(marker).expect("step marker exists");
        let rest = &config[start..];
        let end = rest.find("\n  }\n").expect("step terminator exists") + "\n  }".len();
        rest[..end].to_string()
    }
}
