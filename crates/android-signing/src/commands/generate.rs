//! Generates a release keystore. This command is careful about existing files because signing keys are long-lived.

use crate::android::{self, GenerateKeystore};
use crate::cli::GenerateArgs;
use crate::secrets;
use anyhow::Result;

pub fn run(args: GenerateArgs) -> Result<()> {
    // Overwriting a keystore can break updates for already released Android apps, so it is opt-in.
    if args.output.exists() && !args.force {
        anyhow::bail!("keystore already exists: {}", args.output.display());
    }

    let store_password = if args.generate_passwords {
        secrets::random_password()
    } else {
        args.store_password
            .unwrap_or(secrets::prompt_password("Keystore password")?)
    };

    let key_password = if args.generate_passwords {
        secrets::random_password()
    } else {
        args.key_password.unwrap_or_else(|| store_password.clone())
    };

    android::generate_keystore(
        GenerateKeystore {
            output: &args.output,
            alias: &args.alias,
            store_type: &args.store_type,
            key_alg: &args.key_alg,
            key_size: args.key_size,
            validity: args.validity,
            dname: &args.dname,
            store_password: &store_password,
            key_password: &key_password,
        },
        args.dry_run,
    )?;

    if args.generate_passwords {
        println!("ANDROID_KEYSTORE_PASSWORD={store_password}");
        println!("ANDROID_KEY_ALIAS={}", args.alias);
        println!("ANDROID_KEY_PASSWORD={key_password}");
    }

    if args.print_base64 && !args.dry_run {
        println!(
            "ANDROID_KEYSTORE_BASE64={}",
            android::keystore_base64(&args.output)?
        );
    }

    Ok(())
}
