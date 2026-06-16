//! Local iOS signing file helpers. Base64 is a transport format for CI secrets, not encryption.

use anyhow::{Context, Result};
use base64::Engine;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn file_base64(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn command_available(command: &str) -> bool {
    Command::new(command)
        .arg("-help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn encodes_file_as_base64() {
        let path = env::temp_dir().join(format!(
            "ios-signing-base64-{}-{}.txt",
            std::process::id(),
            line!()
        ));
        fs::write(&path, b"signing").unwrap();

        let encoded = file_base64(&path).unwrap();

        assert_eq!(encoded, "c2lnbmluZw==");
        let _ = fs::remove_file(path);
    }
}
