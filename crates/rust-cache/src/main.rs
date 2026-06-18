//! Standalone compatibility entry point for `rust-cache`.

fn main() -> anyhow::Result<()> {
    rust_cache::run_from(std::env::args_os())
}
