//! Prints CI secret names for an existing Apple certificate and provisioning profile.

use crate::cli::PrintSecretsArgs;
use crate::ios;
use anyhow::Result;

pub fn run(args: PrintSecretsArgs) -> Result<()> {
    println!(
        "IOS_SIGNING_CERTIFICATE_BASE64={}",
        ios::file_base64(&args.certificate)?
    );
    println!(
        "IOS_SIGNING_PROVISIONING_PROFILE_BASE64={}",
        ios::file_base64(&args.provisioning_profile)?
    );
    println!("IOS_SIGNING_CERTIFICATE_PASSWORD=<store this separately>");
    println!("IOS_SIGNING_KEYCHAIN_PASSWORD=<store this separately>");
    println!("APPLE_TEAM_ID=<store this separately>");
    Ok(())
}
