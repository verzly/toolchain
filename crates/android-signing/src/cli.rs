//! Command-line interface for Android signing helpers. This file should define inputs without handling secrets directly.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "android-signing")]
#[command(bin_name = "android-signing")]
#[command(author, version, about = "Android release signing keystore helper")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Generate(GenerateArgs),
    Base64(Base64Args),
    Fingerprint(FingerprintArgs),
    #[command(name = "print-secrets")]
    PrintSecrets(PrintSecretsArgs),
    #[command(name = "write-github-env")]
    WriteGithubEnv(WriteGithubEnvArgs),
    Doctor,
}

#[derive(Args, Debug)]
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

    #[arg(long, default_value = "CN=Android Release, OU=Release, O=Unknown, L=Unknown, ST=Unknown, C=US")]
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
pub struct Base64Args {
    pub path: PathBuf,

    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct FingerprintArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,

    #[arg(long)]
    pub store_password: Option<String>,
}

#[derive(Args, Debug)]
pub struct PrintSecretsArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,
}

#[derive(Args, Debug)]
pub struct WriteGithubEnvArgs {
    pub path: PathBuf,

    #[arg(short, long, default_value = "release-key")]
    pub alias: String,
}
