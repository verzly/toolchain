//! Prints the secret names expected by CI without pretending to know passwords from an existing keystore.

use anyhow::Result;
use crate::android;
use crate::cli::PrintSecretsArgs;

pub fn run(args: PrintSecretsArgs) -> Result<()> {
    println!("ANDROID_KEYSTORE_BASE64={}", android::keystore_base64(&args.path)?);
    println!("ANDROID_KEY_ALIAS={}", args.alias);
    println!("ANDROID_KEYSTORE_PASSWORD=<store this separately>");
    println!("ANDROID_KEY_PASSWORD=<store this separately>");
    Ok(())
}
