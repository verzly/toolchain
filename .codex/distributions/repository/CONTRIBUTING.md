# Contributing

Development for `repository` happens in `verzly/toolchain`.

The public `verzly/repository` repository contains the synchronized distribution surface only:

```text
README.md
CONTRIBUTING.md
action.yml
LICENSE
```

## Development

Clone `verzly/toolchain`, then run checks from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run the executable locally with Cargo:

```sh
cargo run -p repository -- plan
cargo run -p repository -- init --dry-run --skip-mise-use --skip-hk-install
cargo run -p repository -- update --dry-run --skip-mise-use --skip-hk-install
cargo run -p repository -- doctor
```

`doctor` should remain advisory for language setup. It should recommend missing `mise.toml` entries such as `rust@stable`, `aube`, `php`, `composer`, `npm:oxlint`, `npm:oxfmt`, and `npm:vitest`, plus project dependencies such as Rector PHP and Pest PHP.

`repository` should not add package scripts to `package.json` or `composer.json`. The generated `hk.pkl` should call tools directly through the `mise` environment.

When adding central defaults, keep them project-overrideable. Generated config files should be normal repository files, and update operations should preserve local edits unless `--force` is used.

## Documentation

User-facing documentation belongs in `.codex/distributions/repository/README.md`.

Contributor and development workflow details belong in this `CONTRIBUTING.md` file, not in the README.
