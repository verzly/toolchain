//! Standalone compatibility entry point for `android-signing`.

fn main() -> anyhow::Result<()> {
    android_signing::run_from(std::env::args_os())
}
