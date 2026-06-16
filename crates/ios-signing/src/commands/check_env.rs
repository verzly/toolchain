//! Validates that CI provided the iOS signing environment without printing secret values.

use crate::cli::CheckEnvArgs;
use anyhow::Result;

const REQUIRED_SIGNING_ENV: &[&str] = &[
    "IOS_SIGNING_CERTIFICATE_BASE64",
    "IOS_SIGNING_CERTIFICATE_PASSWORD",
    "IOS_SIGNING_PROVISIONING_PROFILE_BASE64",
    "IOS_SIGNING_KEYCHAIN_PASSWORD",
];

pub fn run(args: CheckEnvArgs) -> Result<()> {
    let mut names = REQUIRED_SIGNING_ENV
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();

    if !args.skip_apple_team_id {
        names.push("APPLE_TEAM_ID".to_string());
    }

    for name in args.required {
        if !names.contains(&name) {
            names.push(name);
        }
    }

    let missing = missing_env_vars(&names);
    for name in &names {
        let status = if missing.contains(name) {
            "missing"
        } else {
            "ok"
        };
        println!("{name}: {status}");
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "missing iOS signing environment variables: {}",
            missing.join(", ")
        );
    }

    Ok(())
}

fn missing_env_vars(names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            std::env::var(name.as_str())
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_vars_reports_unset_values() {
        let names = vec!["IOS_SIGNING_TEST_VALUE_THAT_SHOULD_NOT_EXIST".to_string()];

        assert_eq!(missing_env_vars(&names), names);
    }
}
