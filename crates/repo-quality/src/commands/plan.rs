use crate::cli::PlanArgs;
use crate::project::ProjectProfile;
use crate::quality::render_hk_config;
use anyhow::Result;

pub fn run(args: PlanArgs) -> Result<()> {
    let profile = ProjectProfile::detect(args.root, &args.languages, args.js_runner)?;
    println!("Repository: {}", profile.root.display());
    println!("Languages: {:?}", profile.languages);
    println!("JavaScript runner: {:?}", profile.js_runner);
    println!("Package scripts: {:?}", profile.package_scripts);
    println!("Rector detected: {}", profile.has_rector);
    println!("Pest detected: {}", profile.has_pest);
    println!("\n{}", render_hk_config(&profile));
    Ok(())
}
