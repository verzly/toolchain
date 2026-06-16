//! Cache configuration. Defaults keep build output project-local through native Cargo config.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheConfig,
    pub cargo: CargoConfig,
    pub generated: GeneratedConfig,
    pub env: BTreeMap<String, String>,
}

impl Config {
    pub fn default_env() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("GRADLE_USER_HOME".to_string(), "android/gradle".to_string()),
            ("NPM_CONFIG_CACHE".to_string(), "js/npm".to_string()),
            ("YARN_CACHE_FOLDER".to_string(), "js/yarn".to_string()),
            ("PNPM_STORE_PATH".to_string(), "js/pnpm-store".to_string()),
        ])
    }

    fn apply_env_defaults(&mut self) {
        let mut merged = Self::default_env();
        for (key, value) in std::mem::take(&mut self.env) {
            if value.trim().is_empty() {
                merged.remove(&key);
            } else {
                merged.insert(key, value);
            }
        }
        self.env = merged;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            cargo: CargoConfig::default(),
            generated: GeneratedConfig::default(),
            env: Self::default_env(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CacheConfig {
    pub dir: PathBuf,
    pub package: String,
    pub redirect_cargo_home: bool,
    pub redirect_gradle: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(".cache"),
            package: "auto".to_string(),
            redirect_cargo_home: false,
            redirect_gradle: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CargoConfig {
    pub target_dir: String,
}

impl Default for CargoConfig {
    fn default() -> Self {
        Self {
            target_dir: "rust/packages/{package}/target".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct GeneratedConfig {
    pub paths: Vec<PathBuf>,
}

pub fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    let mut config: Config = if is_datarose_config(&value) {
        load_datarose_config(&value)
    } else {
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?
    };
    config.apply_env_defaults();
    Ok(config)
}

fn is_datarose_config(value: &toml::Value) -> bool {
    value.get("rust_cache").is_some()
}

fn load_datarose_config(value: &toml::Value) -> Config {
    let mut config = Config::default();
    let Some(root) = value.get("rust_cache") else {
        return config;
    };

    if let Some(cache) = root.get("cache").and_then(toml::Value::as_table) {
        if let Some(dir) = string_field(cache, "dir") {
            config.cache.dir = PathBuf::from(dir);
        }
        if let Some(package) = string_field(cache, "package") {
            config.cache.package = package;
        }
        if let Some(value) = bool_field(cache, "redirect_cargo_home") {
            config.cache.redirect_cargo_home = value;
        }
        if let Some(value) = bool_field(cache, "redirect_gradle") {
            config.cache.redirect_gradle = value;
        }
    }

    if let Some(cargo) = root.get("cargo").and_then(toml::Value::as_table) {
        if let Some(target_dir) = string_field(cargo, "target_dir") {
            config.cargo.target_dir = target_dir;
        }
    }

    if let Some(generated) = root.get("generated").and_then(toml::Value::as_table) {
        if let Some(paths) = generated.get("paths").and_then(toml::Value::as_array) {
            config.generated.paths = paths
                .iter()
                .filter_map(toml::Value::as_str)
                .map(PathBuf::from)
                .collect();
        }
    }

    if let Some(env) = root.get("env").and_then(toml::Value::as_table) {
        config.env.clear();
        for (key, value) in env {
            if let Some(value) = value.as_str() {
                config.env.insert(key.clone(), value.to_string());
            }
        }
    }

    config
}

fn string_field(table: &toml::value::Table, key: &str) -> Option<String> {
    table.get(key)?.as_str().map(ToOwned::to_owned)
}

fn bool_field(table: &toml::value::Table, key: &str) -> Option<bool> {
    table.get(key)?.as_bool()
}

pub fn write_default_config(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!("config already exists: {}", path.display());
    }
    fs::write(path, render_datarose_default_config())?;
    Ok(())
}

fn render_datarose_default_config() -> String {
    r#"version = 1

[rust_cache.cache]
dir = ".cache"
package = "auto"
redirect_cargo_home = false
redirect_gradle = true

[rust_cache.cargo]
target_dir = "rust/packages/{package}/target"

[rust_cache.generated]
paths = []

[rust_cache.env]
GRADLE_USER_HOME = "android/gradle"
NPM_CONFIG_CACHE = "js/npm"
YARN_CACHE_FOLDER = "js/yarn"
PNPM_STORE_PATH = "js/pnpm-store"
"#
    .to_string()
}

pub fn ensure_config(path: &Path, force: bool) -> Result<Config> {
    if !path.exists() || force {
        write_default_config(path, true)?;
    }
    load(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("rust-cache-{name}-{suffix}.toml"))
    }

    #[test]
    fn missing_config_loads_safe_defaults() {
        let config = load(&temp_path("missing")).expect("load default config");

        assert_eq!(config.cache.dir, PathBuf::from(".cache"));
        assert_eq!(config.cache.package, "auto");
        assert!(!config.cache.redirect_cargo_home);
        assert!(config.cache.redirect_gradle);
        assert_eq!(config.cargo.target_dir, "rust/packages/{package}/target");
        assert!(config.generated.paths.is_empty());
        assert_eq!(config.env["NPM_CONFIG_CACHE"], "js/npm");
        assert_eq!(config.env["PNPM_STORE_PATH"], "js/pnpm-store");
    }

    #[test]
    fn write_default_config_refuses_existing_file_without_force() {
        let path = temp_path("default");
        write_default_config(&path, false).expect("write config");

        let error = write_default_config(&path, false).expect_err("existing config must fail");
        assert!(error.to_string().contains("config already exists"));

        write_default_config(&path, true).expect("force overwrite");
    }

    #[test]
    fn ensure_config_uses_existing_config_without_overwriting() {
        let path = temp_path("existing");
        fs::write(
            &path,
            "[cache]\npackage = \"demo\"\n\n[cargo]\ntarget_dir = \"rust/{package}/target\"\n",
        )
        .expect("write config");

        let config = ensure_config(&path, false).expect("load existing config");

        assert_eq!(config.cache.package, "demo");
        assert_eq!(config.cargo.target_dir, "rust/{package}/target");
    }

    #[test]
    fn datarose_generated_paths_are_loaded() {
        let path = temp_path("generated");
        fs::write(
            &path,
            "[rust_cache.generated]\npaths = [\"apps/desktop/src-tauri/gen/android/app/build\"]\n",
        )
        .expect("write config");

        let config = load(&path).expect("load config");

        assert_eq!(
            config.generated.paths,
            vec![PathBuf::from(
                "apps/desktop/src-tauri/gen/android/app/build"
            )]
        );
    }

    #[test]
    fn custom_env_cache_paths_extend_defaults_when_configured() {
        let path = temp_path("custom-env");
        fs::write(
            &path,
            "[env]\nFOO_CACHE = \"custom/foo\"\nYARN_CACHE_FOLDER = \"\"\n\n[cache]\npackage = \"demo\"\n",
        )
        .expect("write config");

        let config = load(&path).expect("load config");

        assert_eq!(config.env["FOO_CACHE"], "custom/foo");
        assert_eq!(config.env["NPM_CONFIG_CACHE"], "js/npm");
        assert!(!config.env.contains_key("YARN_CACHE_FOLDER"));
    }
}
