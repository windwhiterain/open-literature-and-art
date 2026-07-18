#!/usr/bin/env bash
set -euo pipefail

REPO="windwhiterain/open-literature-and-art"
TAG="nightly"
BINARY="soil"
INSTALL_DIR="${HOME}/.local/bin"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

say() { printf '%b\n' "$1"; }

main() {
  say "${GREEN}==> Installing ${BINARY}...${NC}"

  local url="https://github.com/${REPO}/releases/download/${TAG}/${BINARY}"
  say "    Downloading ${url}"

  mkdir -p "${INSTALL_DIR}"

  local tmp
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  local dest="${tmp}/${BINARY}"

  if command -v curl &>/dev/null; then
    curl -fsSL --progress-bar -o "${dest}" "${url}"
  elif command -v wget &>/dev/null; then
    wget -q --show-progress -O "${dest}" "${url}"
  else
    printf '%b' "${RED}Neither curl nor wget found. Please install one of them.${NC}\n"
    exit 1
  fi

  chmod +x "${dest}"
  mv "${dest}" "${INSTALL_DIR}/${BINARY}"

  say "${GREEN}==> Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}${NC}"

  if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    printf '%b' "${RED}WARNING:${NC} ${INSTALL_DIR} is not in your PATH.\n"
    printf '  Add this to your shell profile (e.g. ~/.bashrc or ~/.zshrc):\n'
    printf '    export PATH="%s:$PATH"\n' "${INSTALL_DIR}"
  else
    say "${GREEN}==> Run '${BINARY} --help' to get started.${NC}"
  fi
}

main
