#!/usr/bin/env bash
# Sync _repos/<tool>/ content into the distribution repository and publish the GitHub Release.
# Usage: scripts/publish-distribution-repo.sh <tool> <version> <assets-root> <repo-dir> <config>
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"
ASSETS_ROOT="${3:?assets root is required}"
REPO_DIR="${4:?distribution repository checkout is required}"
CONFIG="${5:?github-release config path is required}"

REPO_CONTENT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../_repos/${TOOL}" && pwd)"
ASSETS_DIR="$(cd "${ASSETS_ROOT}/${TOOL}" && pwd)"

cd "${REPO_DIR}"
git config user.name "verzly-release-bot"
git config user.email "release-bot@verzly.dev"

# Prepare the release branch in the distribution repository.
# The config lives in the private toolchain so release behavior can evolve centrally.
github-release prepare --config "${CONFIG}" --version "${VERSION}"

# Replace all distribution repository content with the current _repos/<tool>/ snapshot.
# Only .git and .github survive; the latter is intentionally absent from _repos templates.
find . -mindepth 1 -maxdepth 1 \
  ! -name .git \
  ! -name .github \
  -exec rm -rf {} +

cp -R "${REPO_CONTENT}/." .

git add --all
if ! git diff --cached --quiet; then
  git commit -m "chore(release): sync repository files for v${VERSION}"
  git push origin HEAD
fi

# Merge, tag, publish the GitHub Release, and upload assets.
github-release finalize \
  --config "${CONFIG}" \
  --version "${VERSION}" \
  --assets "${ASSETS_DIR}"
