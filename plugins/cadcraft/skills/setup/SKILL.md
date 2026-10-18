---
name: setup
description: One-time setup for the CADCraft plugin. Builds and installs the cadcraft-cli program that the plugin's MCP server runs. Run it when the CADCraft tools are missing or the server fails to start.
disable-model-invocation: true
---

# CADCraft setup

Install `cadcraft-cli`, the program behind the CADCraft MCP server. It is built from source with
Rust's `cargo`, so the first run takes about 5 minutes.

1. Run the installer with the Bash tool. Give it a generous timeout (at least 10 minutes), because it compiles the whole project:

   ```bash
   bash "${CLAUDE_PLUGIN_ROOT}/scripts/install-cadcraft-cli.sh"
   ```

2. Read the result:
   - **Success:** it prints `Installed: <path>` and the version. Tell the user to run `/reload-plugins` (or restart Claude) so the `cadcraft` MCP server starts, and that the CADCraft tools will then be available.
   - **Exit code 2 (Rust missing or too old):** the script prints what to do. Relay it plainly: install Rust from https://rustup.rs (or run `rustup update stable`), open a new terminal, then run `/cadcraft:setup` again. Do not install Rust yourself.
   - **Any other failure:** show the last lines of the error and stop. Common causes are no network access to github.com and a missing C compiler or linker (on Windows, the Visual Studio Build Tools).

3. If the user already has a `cadcraft-cli` binary, no build is needed. They can put it on their `PATH`, or set `CADCRAFT_CLI=/full/path/to/cadcraft-cli`, then run `/reload-plugins`.

To update later, run the same script with `--force` to rebuild from the latest source.
