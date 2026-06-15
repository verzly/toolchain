//! Builds the environment variable plan used by `run` and `env` for non-Cargo cache paths.

use crate::cargo_config;
use crate::config::Config;
use crate::workspace;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct EnvPlan {
    pub workspace_root: PathBuf,
    pub package: String,
    pub cache_root: PathBuf,
    pub cargo_target_dir: PathBuf,
    pub values: BTreeMap<String, String>,
}

impl EnvPlan {
    pub fn build(config: &Config) -> Result<Self> {
        let workspace = workspace::detect()?;
        let package = cargo_config::package_key(config, &workspace);
        let cache_root = workspace.root.join(&config.cache.dir);
        let cargo_target_dir = cargo_config::target_dir_absolute(config, &workspace);
        let mut values = BTreeMap::new();
        values.insert(
            "CARGO_TARGET_DIR".to_string(),
            cargo_target_dir.display().to_string(),
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

        for (key, value) in &config.env {
            values.insert(key.clone(), env_cache_path(&cache_root, value));
        }

        Ok(Self {
            workspace_root: workspace.root,
            package,
            cache_root,
            cargo_target_dir,
            values,
        })
    }

    pub fn print_exports(&self) {
        for (key, value) in &self.values {
            println!("export {key}=\"{value}\"");
        }
    }
}

fn env_cache_path(cache_root: &Path, value: &str) -> String {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path.display().to_string()
    } else {
        cache_root.join(path).display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheConfig, CargoConfig, Config, GeneratedConfig};
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
            cargo: CargoConfig {
                target_dir: "rust/packages/{package}/target".to_string(),
            },
            generated: GeneratedConfig::default(),
            env: BTreeMap::new(),
        };

        let plan = EnvPlan::build(&config).expect("build env plan");

        assert_eq!(plan.package, "demo-package");
        assert!(plan.cache_root.ends_with(".cache-test"));
        assert!(plan
            .values
            .get("CARGO_TARGET_DIR")
            .expect("target dir")
            .replace('\\', "/")
            .ends_with(".cache-test/rust/packages/demo-package/target"));
        assert!(plan.values.contains_key("CARGO_HOME"));
        assert!(!plan.values.contains_key("GRADLE_USER_HOME"));
    }

    #[test]
    fn applies_default_and_custom_language_cache_paths_under_cache_root() {
        let mut config = Config::default();
        config.cache.dir = PathBuf::from(".cache-test");
        config.cache.package = "demo-package".to_string();
        config
            .env
            .insert("FOO_CACHE".to_string(), "foo".to_string());

        let plan = EnvPlan::build(&config).expect("build env plan");

        assert!(plan
            .values
            .get("NPM_CONFIG_CACHE")
            .expect("npm cache")
            .replace('\\', "/")
            .ends_with(".cache-test/js/npm"));
        assert!(plan
            .values
            .get("PNPM_STORE_PATH")
            .expect("pnpm cache")
            .replace('\\', "/")
            .ends_with(".cache-test/js/pnpm-store"));
        assert!(plan
            .values
            .get("FOO_CACHE")
            .expect("custom cache")
            .replace('\\', "/")
            .ends_with(".cache-test/foo"));
    }

    #[test]
    fn keeps_absolute_custom_cache_paths() {
        assert_eq!(
            env_cache_path(Path::new(".cache"), "/tmp/tool-cache"),
            "/tmp/tool-cache"
        );
    }
}
