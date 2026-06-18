//! Command-line interface for Android signing helpers. This file should define inputs without handling secrets directly.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "android-signing")]
#[command(bin_name = "android-signing")]
#[command(
    author,
    version,
    about = "Android release signing keystore helper",
    after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Generate(GenerateArgs),
    Base64(Base64Args),
    Fingerprint(FingerprintArgs),
    #[command(name = "verify-fingerprint")]
    VerifyFingerprint(VerifyFingerprintArgs),
    #[command(name = "print-secrets")]
    PrintSecrets(PrintSecretsArgs),
    #[command(name = "write-github-env")]
    WriteGithubEnv(WriteGithubEnvArgs),
    #[command(name = "check-env")]
    CheckEnv(CheckEnvArgs),
    #[command(
        after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing"
    )]
    Doctor,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct GenerateArgs {
    #[arg(short, long, default_value = "android-release.jks")]
    pub output: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,

    #[arg(long, default_value = "JKS")]
    pub store_type: String,

    #[arg(long, default_value = "RSA")]
    pub key_alg: String,

    #[arg(long, default_value_t = 2048)]
    pub key_size: u32,

    #[arg(long, default_value_t = 10000)]
    pub validity: u32,

    #[arg(
        long,
        default_value = "CN=Android Release, OU=Release, O=Unknown, L=Unknown, ST=Unknown, C=US"
    )]
    pub dname: String,

    #[arg(long)]
    pub store_password: Option<String>,

    #[arg(long)]
    pub key_password: Option<String>,

    #[arg(long, default_value_t = false)]
    pub generate_passwords: bool,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    #[arg(long, default_value_t = false)]
    pub print_base64: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct Base64Args {
    pub path: PathBuf,

    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct FingerprintArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,

    #[arg(long)]
    pub store_password: Option<String>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct VerifyFingerprintArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,

    #[arg(long)]
    pub store_password: Option<String>,

    #[arg(long)]
    pub expected_sha256: String,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct PrintSecretsArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct WriteGithubEnvArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#android-signing")]
pub struct CheckEnvArgs {
    /// Also require ANDROID_SIGNING_CERT_SHA256 for fingerprint verification workflows.
    #[arg(long, default_value_t = false)]
    pub require_fingerprint: bool,

    /// Additional environment variable names to require. Repeatable.
    #[arg(long = "require")]
    pub required: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_generate_defaults() {
        let cli = Cli::parse_from(["android-signing", "generate"]);

        let Commands::Generate(args) = cli.command else {
            panic!("expected generate command");
        };

        assert_eq!(args.output, PathBuf::from("android-release.jks"));
        assert_eq!(args.alias, "release-key");
        assert_eq!(args.store_type, "JKS");
        assert_eq!(args.key_alg, "RSA");
        assert_eq!(args.key_size, 2048);
        assert_eq!(args.validity, 10000);
        assert!(!args.force);
        assert!(!args.dry_run);
    }

    #[test]
    fn parses_verify_fingerprint_options() {
        let cli = Cli::parse_from([
            "android-signing",
            "verify-fingerprint",
            "release.jks",
            "--alias",
            "release-key",
            "--expected-sha256",
            "AA:BB",
        ]);

        let Commands::VerifyFingerprint(args) = cli.command else {
            panic!("expected verify-fingerprint command");
        };

        assert_eq!(args.path, PathBuf::from("release.jks"));
        assert_eq!(args.alias, "release-key");
        assert_eq!(args.expected_sha256, "AA:BB");
    }

    #[test]
    fn parses_check_env_options() {
        let cli = Cli::parse_from([
            "android-signing",
            "check-env",
            "--require-fingerprint",
            "--require",
            "ANDROID_KEYSTORE_PATH",
        ]);

        let Commands::CheckEnv(args) = cli.command else {
            panic!("expected check-env command");
        };

        assert!(args.require_fingerprint);
        assert_eq!(args.required, vec!["ANDROID_KEYSTORE_PATH"]);
    }
}
