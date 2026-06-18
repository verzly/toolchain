//! Standalone compatibility entry point for `cargo-release`.

fn main() -> anyhow::Result<()> {
    cargo_release::run_from(std::env::args_os())
}
