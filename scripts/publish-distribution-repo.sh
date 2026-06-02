#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"
ASSETS_ROOT="${3:?assets root is required}"
REPO_DIR="${4:?distribution repository checkout is required}"

TEMPLATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../distribution/${TOOL}" && pwd)"
CONFIG="${TEMPLATE_DIR}/github-release.toml"
ASSETS_DIR="$(cd "${ASSETS_ROOT}/${TOOL}" && pwd)"

cd "${REPO_DIR}"
git config user.name "verzly-release-bot"
git config user.email "release-bot@verzly.dev"

# The config is read from the private toolchain template so release behavior can evolve
# without requiring a direct config commit in the distribution repository first.
github-release prepare --config "${CONFIG}" --version "${VERSION}"

# Keep the distribution repository intentionally small: metadata, README, action.yml,
# workflows, VERSION, and license. The Rust source stays in verzly/toolchain.
find . -mindepth 1 -maxdepth 1 \
  ! -name .git \
  ! -name .github \
  -exec rm -rf {} +
rm -rf .github
cp -R "${TEMPLATE_DIR}/." .

git add --all
if ! git diff --cached --quiet; then
  git commit -m "chore(release): sync distribution files for v${VERSION}"
  git push origin HEAD
fi

github-release finalize --config "${CONFIG}" --version "${VERSION}" --assets "${ASSETS_DIR}"
