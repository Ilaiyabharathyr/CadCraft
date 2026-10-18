#!/usr/bin/env bash
# One-time setup: install `cadcraft-cli`, the program behind the CADCraft MCP server.
#
# It first downloads a prebuilt binary from the project's GitHub release (a few seconds) and
# checks it against the release's SHA256SUMS.txt. If there is no binary for this platform, or the
# download fails, it builds from source with cargo instead (about 5 minutes, needs Rust).
#
#   install-cadcraft-cli.sh [--force] [--from-source] [extra cargo-install args]
#
#   --force        reinstall even if cadcraft-cli is already installed
#   --from-source  skip the download and build with cargo
#
# Environment:
#   CADCRAFT_HOME           install root (the binary lands in $CADCRAFT_HOME/bin). Default: ~/.cadcraft
#   CADCRAFT_REPO_URL       GitHub repository. Default: https://github.com/ihayashi/cadcraft
#   CADCRAFT_DOWNLOAD_BASE  where the release files live. Default: $CADCRAFT_REPO_URL/releases/latest/download
#   CADCRAFT_GIT_REF        branch, tag or commit to build from source (implies --from-source)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=cadcraft-env.sh
. "$here/cadcraft-env.sh"

repo_url="${CADCRAFT_REPO_URL:-https://github.com/ihayashi/cadcraft}"
download_base="${CADCRAFT_DOWNLOAD_BASE:-$repo_url/releases/latest/download}"
min_rust="1.90"

force=0
from_source=0
extra=0
cargo_args=()
for arg in "$@"; do
  case "$arg" in
    --force) force=1 ;;
    --from-source) from_source=1 ;;
    *) cargo_args+=("$arg"); extra=1 ;;
  esac
done
if [ -n "${CADCRAFT_GIT_REF:-}" ]; then
  from_source=1
fi

tmp_dir=""
cleanup() {
  if [ -n "$tmp_dir" ]; then rm -rf "$tmp_dir"; fi
}
trap cleanup EXIT

# The Rust target triple of a prebuilt binary for this machine, or return 1 if there is none.
detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      case "$arch" in
        x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
        aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
        *) return 1 ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64 | aarch64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) return 1 ;;
      esac
      ;;
    MINGW* | MSYS* | CYGWIN*)
      case "$arch" in
        x86_64 | amd64) echo "x86_64-pc-windows-msvc" ;;
        *) return 1 ;;
      esac
      ;;
    *) return 1 ;;
  esac
}

# fetch URL DEST: download a file with curl or wget.
fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 2 -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    echo "Neither curl nor wget is available to download files." >&2
    return 127
  fi
}

# sha256_of FILE: print the SHA-256 of a file.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "No sha256sum or shasum available to verify the download." >&2
    return 1
  fi
}

# Download, verify and install the prebuilt binary. Returns 1 (after saying why) on any problem,
# and leaves nothing half-installed.
install_prebuilt() {
  local target asset expected actual bin installed
  target="$(detect_target)" || {
    echo "No prebuilt binary for $(uname -s) $(uname -m)." >&2
    return 1
  }
  asset="cadcraft-cli-$target.tar.gz"
  tmp_dir="$(mktemp -d)"

  echo "Downloading the prebuilt $asset from $download_base ..."
  fetch "$download_base/$asset" "$tmp_dir/$asset" || {
    echo "Could not download $asset (no release published yet, or no network)." >&2
    return 1
  }
  fetch "$download_base/SHA256SUMS.txt" "$tmp_dir/SHA256SUMS.txt" || {
    echo "Could not download SHA256SUMS.txt, so the binary cannot be verified." >&2
    return 1
  }

  expected="$(awk -v a="$asset" '$2 == a || $2 == "*" a {print $1; exit}' "$tmp_dir/SHA256SUMS.txt")"
  actual="$(sha256_of "$tmp_dir/$asset")" || return 1
  if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    echo "Checksum mismatch for $asset (expected '${expected:-none listed}', got '$actual'). Not installing it." >&2
    return 1
  fi
  echo "Checksum OK."

  mkdir -p "$tmp_dir/unpacked" "$CADCRAFT_HOME/bin"
  tar -xzf "$tmp_dir/$asset" -C "$tmp_dir/unpacked" || {
    echo "Could not unpack $asset." >&2
    return 1
  }
  bin=""
  for installed in cadcraft-cli cadcraft-cli.exe; do
    if [ -f "$tmp_dir/unpacked/$installed" ]; then bin="$installed"; fi
  done
  if [ -z "$bin" ]; then
    echo "$asset does not contain cadcraft-cli." >&2
    return 1
  fi

  cp "$tmp_dir/unpacked/$bin" "$CADCRAFT_HOME/bin/$bin"
  chmod +x "$CADCRAFT_HOME/bin/$bin"
  # A glibc binary will not run on musl systems such as Alpine: make sure it really starts.
  if ! "$CADCRAFT_HOME/bin/$bin" --version >/dev/null 2>&1; then
    echo "The downloaded binary does not run on this system." >&2
    rm -f "$CADCRAFT_HOME/bin/$bin"
    return 1
  fi
  return 0
}

# Build and install from source with cargo. Exits the script when Rust is missing or too old.
install_from_source() {
  if ! command -v cargo >/dev/null 2>&1; then
    cat >&2 <<EOF
Could not install a prebuilt cadcraft-cli, and Rust (cargo) is not installed to build it from source.

Either install Rust from https://rustup.rs (Rust $min_rust or newer), open a new terminal and run
this setup again, or put a cadcraft-cli binary on your PATH (or set CADCRAFT_CLI to its full path).
EOF
    exit 2
  fi

  # Compare "major.minor" of rustc against the minimum.
  local rustc_version have_major rest have_minor want_major want_minor
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

  local git_ref_args=() force_args=()
  if [ -n "${CADCRAFT_GIT_REF:-}" ]; then
    git_ref_args=(--rev "$CADCRAFT_GIT_REF")
  fi
  if [ "$force" -eq 1 ]; then
    force_args=(--force)
  fi

  echo "Building cadcraft-cli from $repo_url with Rust $rustc_version."
  echo "This downloads dependencies and compiles in release mode: expect about 5 minutes."
  mkdir -p "$CADCRAFT_HOME"
  # ${arr[@]+...} keeps an empty array from tripping `set -u` on bash 3.2 (macOS).
  cargo install --git "$repo_url" \
    ${git_ref_args[@]+"${git_ref_args[@]}"} \
    ${force_args[@]+"${force_args[@]}"} \
    ${cargo_args[@]+"${cargo_args[@]}"} \
    --locked --root "$CADCRAFT_HOME" cadcraft-cli
}

if cli="$(cadcraft_find_cli)" && [ "$force" -eq 0 ] && [ "$from_source" -eq 0 ] && [ "$extra" -eq 0 ]; then
  echo "cadcraft-cli is already installed: $cli"
  "$cli" --version
  echo "Nothing to do. To reinstall the latest version, run this script again with --force."
  exit 0
fi

if [ "$from_source" -eq 0 ]; then
  if install_prebuilt; then
    echo "Installed the prebuilt binary."
  else
    echo "Falling back to building from source."
    install_from_source
  fi
else
  install_from_source
fi

if cli="$(cadcraft_find_cli)"; then
  echo
  echo "Installed: $cli"
  "$cli" --version
  echo "Now run /reload-plugins (or restart Claude) so the CADCraft MCP server starts."
else
  echo "Installation finished but cadcraft-cli was not found under $CADCRAFT_HOME/bin." >&2
  exit 1
fi
