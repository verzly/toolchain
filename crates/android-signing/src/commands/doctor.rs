//! Checks local tooling required for Android signing. Keep this quick and understandable.

use crate::android;
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("keytool: {}", if android::keytool_available() { "ok" } else { "missing" });
    Ok(())
}
