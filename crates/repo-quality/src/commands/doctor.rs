use crate::cli::DoctorArgs;
use crate::project::ProjectProfile;
use crate::shell;
use anyhow::{bail, Result};

pub fn run(args: DoctorArgs) -> Result<()> {
    let profile = ProjectProfile::detect(args.root, &[], crate::cli::JsRunnerArg::Auto)?;
    let mut failures = Vec::new();

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
