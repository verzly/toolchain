//! Checks local tooling required for Android signing. Keep this quick and understandable.

use anyhow::Result;
use crate::android;

pub fn run() -> Result<()> {
    println!("keytool: {}", if android::keytool_available() { "ok" } else { "missing" });
    Ok(())
}
