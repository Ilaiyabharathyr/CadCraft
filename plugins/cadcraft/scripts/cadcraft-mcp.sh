#!/usr/bin/env bash
# Starts the CADCraft MCP server (headless, JSON-RPC over stdio) for Claude.
#
# stdout is the MCP channel, so this script must never print to it: every message goes to stderr,
# and the server replaces this process with `exec`.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=cadcraft-env.sh
. "$here/cadcraft-env.sh"

if cli="$(cadcraft_find_cli)"; then
  exec "$cli" mcp
fi

cat >&2 <<'EOF'
cadcraft-cli was not found, so the CADCraft MCP server cannot start.

One-time setup: in Claude, run  /cadcraft:setup
(it downloads a prebuilt cadcraft-cli from https://github.com/ihayashi/cadcraft, or builds it
with cargo if there is none for your system),
then run /reload-plugins or restart Claude.

Already have a binary somewhere? Put it on PATH or set CADCRAFT_CLI=/full/path/to/cadcraft-cli.
EOF
exit 1
