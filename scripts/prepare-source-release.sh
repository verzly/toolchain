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
MANIFEST="${ROOT_DIR}/crates/${TOOL}/Cargo.toml"
TAG="${TOOL}-v${VERSION}"
RELEASE_BRANCH="release/${TAG}"

if [[ ! -f "${MANIFEST}" ]]; then
  echo "Unknown tool or missing manifest: ${MANIFEST}" >&2
  exit 1
fi

cd "${ROOT_DIR}"

git diff --quiet
git diff --cached --quiet

git fetch origin "${TARGET_BRANCH}"
if git ls-remote --exit-code --heads origin "${RELEASE_BRANCH}" >/dev/null 2>&1; then
  echo "Release branch already exists on origin: ${RELEASE_BRANCH}" >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null || git ls-remote --exit-code --tags origin "${TAG}" >/dev/null 2>&1; then
  echo "Source tag already exists: ${TAG}" >&2
  exit 1
fi

git checkout -B "${RELEASE_BRANCH}" "origin/${TARGET_BRANCH}"

python3 - "${MANIFEST}" "${VERSION}" <<'PY'
from pathlib import Path
import re
import sys

manifest = Path(sys.argv[1])
version = sys.argv[2]
text = manifest.read_text()

match = re.search(r'(?ms)^\[package\]\n(?P<body>.*?)(?=^\[|\Z)', text)
if not match:
    raise SystemExit(f"missing [package] section in {manifest}")

body = match.group('body')
new_body, count = re.subn(r'(?m)^version\s*=\s*"[^"]*"\s*$', f'version = "{version}"', body, count=1)
if count != 1:
    raise SystemExit(f"missing package version field in {manifest}")

text = text[:match.start('body')] + new_body + text[match.end('body'):]
manifest.write_text(text)
PY

# Keep Cargo.lock in sync when the repository has one. The workspace may intentionally
# start without a lock file, so absence is not treated as an error.
if [[ -f Cargo.lock ]]; then
  cargo generate-lockfile
fi

git add "${MANIFEST}"
if [[ -f Cargo.lock ]]; then
  git add Cargo.lock
fi

if git diff --cached --quiet; then
  echo "No source version changes were needed for ${TOOL} v${VERSION}."
else
  git commit -m "chore(release): prepare ${TOOL} v${VERSION}"
fi

git push --set-upstream origin "${RELEASE_BRANCH}"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "tool=${TOOL}"
    echo "version=${VERSION}"
    echo "tag=${TAG}"
    echo "release_branch=${RELEASE_BRANCH}"
  } >> "${GITHUB_OUTPUT}"
fi

printf 'Prepared source release branch %s for %s v%s\n' "${RELEASE_BRANCH}" "${TOOL}" "${VERSION}"
