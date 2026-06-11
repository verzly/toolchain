use crate::cli::PlanArgs;
use crate::project::ProjectProfile;
use crate::quality::render_hk_config;
use crate::standards;
use crate::workflow::render_test_workflow;
use anyhow::Result;

pub fn run(args: PlanArgs) -> Result<()> {
    let profile = ProjectProfile::detect(
        args.root,
        args.config,
        args.workspace,
        &args.languages,
        args.js_runner,
    )?;
    println!("Repository: {}", profile.root.display());
    println!("Workspace: {}", profile.workspace_display());
    println!("Languages: {:?}", profile.languages);
    println!("JavaScript runner: {:?}", profile.js_runner);
    println!("Rector detected: {}", profile.has_rector);
    println!("Pest detected: {}", profile.has_pest);
    println!("Config: {}", profile.config_path.display());
    println!("mise.toml detected: {}", profile.has_mise_toml);
    println!("Mise tools: {:?}", profile.mise_tools);
    let recommendations = profile.missing_mise_tools();
    if !recommendations.is_empty() {
        println!("Mise recommendations:");
        for recommendation in recommendations {
            println!("- {} ({})", recommendation.command(), recommendation.reason);
        }
    }
    println!("Managed style files:");
    for file in standards::style_files(&profile, false) {
        println!("- {}", file.path.display());
    }
    println!(
        "- {}",
        profile.root.join(".github/workflows/test.yml").display()
    );
    println!("\n--- hk.pkl ---\n{}", render_hk_config(&profile));
    println!(
        "\n--- .github/workflows/test.yml ---\n{}",
        render_test_workflow(&profile)
    );
    if profile.release_enabled() {
        println!("Release targets:");
        for target in &profile.stored_config.release.targets {
            println!(
                "- {} | path={} | strategy={} | workflow={} | repository={}",
                target.name, target.path, target.strategy, target.workflow, target.repository
            );
        }
    }
    Ok(())
}
