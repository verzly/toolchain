# Repository instructions

This repository is the single source and release surface for the Verzly toolchain.

Do not reintroduce `.verzly/distributions`, distribution repository synchronization, or release workflows that publish to separate `verzly/<tool>` repositories. Public usage must go through `verzly/toolchain`, either with the root `action.yml`, subpath actions under `actions/`, or the `verzly` release assets.

Keep the tool implementation modular:

- `crates/verzly` is the unified executable and should mostly dispatch.
- Each tool crate owns its CLI contract, command logic, tests, and reusable `run_from` entrypoint.
- Standalone binaries remain compatibility wrappers.

Action quality rules:

- Do not print secret values.
- Prefer clear inputs, clear outputs, and safe optional mode for signing checks.
- Use `actions/_shared/install-verzly.sh` for installing the released binary.
- Keep iOS signing workflows robust enough to report missing secrets without failing when optional mode is enabled.

Release rules:

- The only public release target is `verzly` in `verzly/toolchain`.
- No `DISTRIBUTION_REPO_TOKEN` should be required for the main release path.
- `datarose.toml` is the source of truth for release files, cache layout, and build targets.
