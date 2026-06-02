#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"

case "${VERSION}" in
  v*) VERSION="${VERSION#v}" ;;
esac

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAG="${TOOL}-v${VERSION}"
RELEASE_BRANCH="release/${TAG}"

cd "${ROOT_DIR}"

if git ls-remote --exit-code --heads origin "${RELEASE_BRANCH}" >/dev/null 2>&1; then
  git push origin --delete "${RELEASE_BRANCH}"
  echo "Deleted source release branch ${RELEASE_BRANCH}."
else
  echo "Source release branch does not exist on origin: ${RELEASE_BRANCH}."
fi

git branch -D "${RELEASE_BRANCH}" >/dev/null 2>&1 || true
