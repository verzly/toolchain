//! Verifies that a keystore still matches the expected release signing certificate.

use crate::android;
use crate::cli::VerifyFingerprintArgs;
use crate::secrets;
use anyhow::Result;

pub fn run(args: VerifyFingerprintArgs) -> Result<()> {
    let password = match args.store_password {
        Some(password) => password,
        None => secrets::prompt_password("Keystore password")?,
    };

    android::verify_sha256_fingerprint(&args.path, &args.alias, &password, &args.expected_sha256)?;
    println!("fingerprint: ok");
    Ok(())
}
