//! Prints the signing certificate fingerprint so releases can be checked against the expected key.

use anyhow::Result;
use crate::android;
use crate::cli::FingerprintArgs;
use crate::secrets;

pub fn run(args: FingerprintArgs) -> Result<()> {
    let password = args.store_password.unwrap_or(secrets::prompt_password("Keystore password")?);
    android::fingerprint(&args.path, &args.alias, &password)
}
