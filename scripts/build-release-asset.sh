#!/usr/bin/env bash
# Build a single release asset for the current platform and place it in .release-assets/<tool>/.
# Usage: scripts/build-release-asset.sh <tool> <version>
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"

case "${RUNNER_OS:-$(uname -s)}" in
  Linux)
    EXT=""
    ARCH="$(uname -m)"
    case "${ARCH}" in
      x86_64|amd64)   HOST="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64)  HOST="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported Linux architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  macOS|Darwin)
    EXT=""
    ARCH="$(uname -m)"
    case "${ARCH}" in
      x86_64|amd64)   HOST="x86_64-apple-darwin" ;;
      aarch64|arm64)  HOST="aarch64-apple-darwin" ;;
      *) echo "Unsupported macOS architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  Windows*)
    EXT=".exe"
    ARCH="${PROCESSOR_ARCHITECTURE:-AMD64}"
    case "${ARCH}" in
      AMD64|amd64|x86_64) HOST="x86_64-pc-windows-msvc" ;;
      ARM64|arm64|aarch64) HOST="aarch64-pc-windows-msvc" ;;
      *) echo "Unsupported Windows architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported runner OS: ${RUNNER_OS:-$(uname -s)}" >&2
    exit 1
    ;;
esac

ASSET_DIR=".release-assets/${TOOL}"
mkdir -p "${ASSET_DIR}"

# Build the binary directly on the host runner.
# cargo-release is not bootstrapped here because the release workflow already builds it
# in CI when needed; this script keeps the build simple and auditable.
cargo build --release -p "${TOOL}"

SOURCE="target/release/${TOOL}${EXT}"
if [ ! -f "${SOURCE}" ]; then
  echo "Expected binary not found: ${SOURCE}" >&2
  exit 1
fi

TARGET="${ASSET_DIR}/${TOOL}-v${VERSION}-${HOST}${EXT}"
cp "${SOURCE}" "${TARGET}"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "${ASSET_DIR}" && sha256sum "$(basename "${TARGET}")" > "$(basename "${TARGET}").sha256")
elif command -v shasum >/dev/null 2>&1; then
  (cd "${ASSET_DIR}" && shasum -a 256 "$(basename "${TARGET}")" > "$(basename "${TARGET}").sha256")
else
  echo "No checksum command found; skipping checksum for ${TARGET}" >&2
fi

echo "Built: ${TARGET}"
