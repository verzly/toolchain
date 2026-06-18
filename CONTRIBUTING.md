# Contributing

Source changes, action surfaces, documentation, and releases for Verzly live in this repository.

Use one repository model: `verzly/toolchain` publishes the `verzly` executable and the GitHub Actions under `action.yml` and `actions/<tool>/action.yml`. Do not add `.verzly/distributions`, distribution sync workflows, or separate public repository copies.

Before opening a PR, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Keep crates modular. Shared command behavior should live in each crate's library entrypoint, while `crates/verzly` should only dispatch to those entrypoints.

For action changes, keep inputs explicit, avoid printing secret values, provide useful outputs, and prefer optional preflight behavior for signing-related workflows.
