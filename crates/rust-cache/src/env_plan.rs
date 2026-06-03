//! Builds the environment variable plan used by `run` and `env`. This is the core behavior of the tool.

use crate::config::{CacheConfig, Config};
use crate::workspace;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;
// The plan is built before the command runs so contributors can inspect paths without side effects.
#[derive(Clone, Debug)]
pub struct EnvPlan {
    pub workspace_root: PathBuf,
    pub package: String,
    pub cache_root: PathBuf,
    pub values: BTreeMap<String, String>,
}

impl EnvPlan {
    pub fn build(config: &Config) -> Result<Self> {
        let workspace = workspace::detect()?;
        // `auto` keeps the config short, while an explicit package key keeps monorepo cache paths stable.
        let package = if config.cache.package == "auto" {
            workspace.package.unwrap_or_else(|| "workspace".to_string())
        } else {
            config.cache.package.clone()
        };

        let cache_root = workspace.root.join(&config.cache.dir);
        let package_root = cache_root.join("rust").join("packages").join(&package);
        let mut values = BTreeMap::new();
        values.insert(
            "CARGO_TARGET_DIR".to_string(),
            package_root.join("target").display().to_string(),
        );

        if config.cache.redirect_cargo_home {
            values.insert(
                "CARGO_HOME".to_string(),
                cache_root
                    .join("rust")
                    .join("cargo-home")
                    .display()
                    .to_string(),
            );
        }

        if config.cache.redirect_gradle {
            values.insert(
                "GRADLE_USER_HOME".to_string(),
                cache_root
                    .join("android")
                    .join("gradle")
                    .display()
                    .to_string(),
            );
        }

        Ok(Self {
            workspace_root: workspace.root,
            package,
            cache_root,
            values,
        })
    }

    pub fn print_exports(&self) {
        for (key, value) in &self.values {
            println!("export {key}=\"{value}\"");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheConfig, Config};
    use std::path::PathBuf;

    #[test]
    fn builds_project_local_cache_environment() {
        let config = Config {
            cache: CacheConfig {
                dir: PathBuf::from(".cache-test"),
                package: "demo-package".to_string(),
                redirect_cargo_home: true,
                redirect_gradle: false,
            },
        };

        let plan = EnvPlan::build(&config).expect("build env plan");

        assert_eq!(plan.package, "demo-package");
        assert!(plan.cache_root.ends_with(".cache-test"));
        assert!(plan
            .values
            .get("CARGO_TARGET_DIR")
            .expect("target dir")
            .ends_with(".cache-test/rust/packages/demo-package/target"));
        assert!(plan.values.contains_key("CARGO_HOME"));
        assert!(!plan.values.contains_key("GRADLE_USER_HOME"));
    }
}
