//! Project inventory command.

use crate::cli::{JsRunnerArg, ProjectsArgs};
use crate::output::{
    compact_table, field, header_cell, js_runner_summary, language_summary, plain_cell,
    print_release_target_table, section, style_compact_columns, value_cell,
};
use crate::project::{detect_cargo_packages, ProjectProfile};
use anyhow::Result;

pub fn run(args: ProjectsArgs) -> Result<()> {
    let profile = ProjectProfile::detect(args.root, args.config, None, &[], JsRunnerArg::Auto)?;
    section("Projects");
    field("Root", profile.root.display().to_string());
    field("Workspace", profile.workspace_display());
    field("Languages", language_summary(&profile));
    field("JavaScript runner", js_runner_summary(&profile));
    println!();

    let packages = detect_cargo_packages(&profile.root)?;
    if packages.is_empty() {
        println!("Cargo packages: -");
    } else {
        println!("Cargo packages:");
        let mut table = compact_table();
        table.set_header([header_cell("Package"), header_cell("Release target")]);
        style_compact_columns(&mut table);
        for package in packages {
            let release_target = profile
                .stored_config
                .release
                .targets
                .iter()
                .find(|target| target.cargo_package == package)
                .map(|target| target.name.as_str())
                .unwrap_or("-");
            table.add_row([value_cell(package, 24), plain_cell(release_target, 24)]);
        }
        println!("{table}");
    }

    println!();
    print_release_target_table(&profile);
    Ok(())
}
