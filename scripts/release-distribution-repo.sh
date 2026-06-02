#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"
ASSETS_ROOT="${3:?assets root is required}"
REPO_DIR="${4:?distribution repository checkout is required}"
CONFIG="${5:?github-release config path is required}"
shift 5

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ASSETS_DIR="$(cd "${ASSETS_ROOT}/${TOOL}" && pwd)"
REPO_PATH="$(cd "${REPO_DIR}" && pwd)"
CONFIG_PATH="$(cd "$(dirname "${CONFIG}")" && pwd)/$(basename "${CONFIG}")"

cd "${REPO_PATH}"
git config user.name "verzly-release-bot"
git config user.email "release-bot@verzly.dev"

github-release prepare --config "${CONFIG_PATH}" --version "${VERSION}"

# Distribution repository contents are not part of verzly/toolchain.
# In the handoff ZIP, they are included as a sibling `_repos/` directory only for convenience.
# CI releases skip content sync by default and only publish release assets/notes to the already-existing public repo.
# For a local/manual sync from the handoff bundle, set:
#   DISTRIBUTION_REPO_CONTENT_ROOT=../_repos
if [ -n "${DISTRIBUTION_REPO_CONTENT_ROOT:-}" ]; then
  "${ROOT_DIR}/scripts/sync-repo-template.sh" "${TOOL}" "${REPO_PATH}"

  git add --all
  if ! git diff --cached --quiet; then
    git commit -m "chore(release): sync distribution files for v${VERSION}"
    git push origin HEAD
  else
    echo "No distribution file changes for ${TOOL} v${VERSION}."
  fi
else
  echo "DISTRIBUTION_REPO_CONTENT_ROOT is not set; skipping distribution repository file sync."
fi

github-release finalize --config "${CONFIG_PATH}" --version "${VERSION}" --assets "${ASSETS_DIR}" "$@"
