#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "setup-cargo-test-env: no Linux package setup required on $(uname -s)"
  exit 0
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "setup-cargo-test-env: unsupported Linux package manager; install pkg-config and libasound2-dev manually" >&2
  exit 1
fi

apt_get=(apt-get)
if [[ "$(id -u)" -ne 0 ]]; then
  if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
    apt_get=(sudo apt-get)
  else
    echo "setup-cargo-test-env: need root privileges to install packages; run as root or configure passwordless sudo" >&2
    exit 1
  fi
fi

packages=()
for package in pkg-config libasound2-dev; do
  if ! dpkg -s "$package" >/dev/null 2>&1; then
    packages+=("$package")
  fi
done

if [[ ${#packages[@]} -eq 0 ]]; then
  echo "setup-cargo-test-env: required packages are already installed"
  exit 0
fi

"${apt_get[@]}" update
"${apt_get[@]}" install -y "${packages[@]}"
