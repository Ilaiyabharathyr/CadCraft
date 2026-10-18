#!/usr/bin/env bash
# Shared helpers for the CADCraft plugin scripts. Meant to be sourced, not run.

# Where `install-cadcraft-cli.sh` puts the binary (cargo's --root). Override with CADCRAFT_HOME.
CADCRAFT_HOME="${CADCRAFT_HOME:-$HOME/.cadcraft}"

# Print the path of an installed cadcraft-cli and return 0, or return 1 if there is none.
# Search order: $CADCRAFT_CLI (explicit override), PATH, the plugin's own install dir,
# the plugin data dir, and cargo's default bin dir.
cadcraft_find_cli() {
  local name dir found
  if [ -n "${CADCRAFT_CLI:-}" ] && [ -x "$CADCRAFT_CLI" ]; then
    printf '%s\n' "$CADCRAFT_CLI"
    return 0
  fi
  for name in cadcraft-cli cadcraft-cli.exe; do
    if found="$(command -v "$name" 2>/dev/null)" && [ -n "$found" ]; then
      printf '%s\n' "$found"
      return 0
    fi
    for dir in "$CADCRAFT_HOME/bin" "${CLAUDE_PLUGIN_DATA:+$CLAUDE_PLUGIN_DATA/bin}" "${CARGO_HOME:-$HOME/.cargo}/bin"; do
      if [ -n "$dir" ] && [ -x "$dir/$name" ]; then
        printf '%s\n' "$dir/$name"
        return 0
      fi
    done
  done
  return 1
}
