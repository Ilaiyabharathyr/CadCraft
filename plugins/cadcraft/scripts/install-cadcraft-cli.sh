#!/usr/bin/env bash
# One-time setup: build and install `cadcraft-cli` from the CADCraft repository with cargo.
#
#   install-cadcraft-cli.sh [extra cargo-install args, e.g. --force]
#
# Environment:
#   CADCRAFT_HOME      install root (the binary lands in $CADCRAFT_HOME/bin). Default: ~/.cadcraft
#   CADCRAFT_REPO_URL  git repository to build. Default: https://github.com/ihayashi/cadcraft
#   CADCRAFT_GIT_REF   optional branch, tag or commit to build instead of the default branch
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=cadcraft-env.sh
. "$here/cadcraft-env.sh"

repo_url="${CADCRAFT_REPO_URL:-https://github.com/ihayashi/cadcraft}"
min_rust="1.90"

if cli="$(cadcraft_find_cli)" && [ "$#" -eq 0 ]; then
  echo "cadcraft-cli is already installed: $cli"
  "$cli" --version
  echo "Nothing to do. To rebuild from the latest source, run this script again with --force."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  cat >&2 <<EOF
Rust (cargo) is not installed, and CADCraft is built from source.

Install it from https://rustup.rs (one command), open a new terminal, then run this setup again.
Rust $min_rust or newer is required.
EOF
  exit 2
fi

# Compare "major.minor" of rustc against the minimum.
rustc_version="$(rustc --version | awk '{print $2}')"
have_major="${rustc_version%%.*}"
rest="${rustc_version#*.}"
have_minor="${rest%%.*}"
want_major="${min_rust%%.*}"
want_minor="${min_rust#*.}"
if [ "$have_major" -lt "$want_major" ] || { [ "$have_major" -eq "$want_major" ] && [ "$have_minor" -lt "$want_minor" ]; }; then
  echo "Rust $rustc_version is too old: CADCraft needs $min_rust or newer. Run: rustup update stable" >&2
  exit 2
fi

git_ref_args=()
if [ -n "${CADCRAFT_GIT_REF:-}" ]; then
  git_ref_args=(--rev "$CADCRAFT_GIT_REF")
fi

echo "Building cadcraft-cli from $repo_url with Rust $rustc_version."
echo "This downloads dependencies and compiles in release mode: expect about 5 minutes the first time."
mkdir -p "$CADCRAFT_HOME"
# ${arr[@]+...} keeps an empty array from tripping `set -u` on bash 3.2 (macOS).
cargo install --git "$repo_url" ${git_ref_args[@]+"${git_ref_args[@]}"} --locked --root "$CADCRAFT_HOME" "$@" cadcraft-cli

if cli="$(cadcraft_find_cli)"; then
  echo
  echo "Installed: $cli"
  "$cli" --version
  echo "Now run /reload-plugins (or restart Claude) so the CADCraft MCP server starts."
else
  echo "Build finished but cadcraft-cli was not found under $CADCRAFT_HOME/bin." >&2
  exit 1
fi
