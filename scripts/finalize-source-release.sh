#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"
TARGET_BRANCH="${3:-master}"

case "${VERSION}" in
  v*) VERSION="${VERSION#v}" ;;
esac

if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid SemVer version: ${VERSION}" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAG="${TOOL}-v${VERSION}"
RELEASE_BRANCH="release/${TAG}"

cd "${ROOT_DIR}"

git diff --quiet
git diff --cached --quiet

git fetch origin "${TARGET_BRANCH}" "${RELEASE_BRANCH}"
git checkout -B "${TARGET_BRANCH}" "origin/${TARGET_BRANCH}"
git merge --no-ff "origin/${RELEASE_BRANCH}" -m "chore(release): merge ${TOOL} v${VERSION}"
git push origin "${TARGET_BRANCH}"

git tag -a "${TAG}" -m "${TOOL} v${VERSION}"
git push origin "${TAG}"

git push origin --delete "${RELEASE_BRANCH}"

git branch -D "${RELEASE_BRANCH}" >/dev/null 2>&1 || true

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "tool=${TOOL}"
    echo "version=${VERSION}"
    echo "tag=${TAG}"
  } >> "${GITHUB_OUTPUT}"
fi

printf 'Finalized source release %s and merged %s into %s\n' "${TAG}" "${RELEASE_BRANCH}" "${TARGET_BRANCH}"
