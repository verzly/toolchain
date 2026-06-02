//! Host platform naming used by release assets and GitHub Actions wrappers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HostPlatform {
    pub os: String,
    pub arch: String,
}

impl HostPlatform {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }

    pub fn asset_label(&self) -> String {
        let os = match self.os.as_str() {
            "macos" => "macos",
            "windows" => "windows",
            "linux" => "linux",
            other => other,
        };
        let arch = match self.arch.as_str() {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => other,
        };
        format!("{os}-{arch}")
    }
}
