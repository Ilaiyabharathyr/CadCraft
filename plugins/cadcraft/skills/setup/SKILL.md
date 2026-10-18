---
name: setup
description: One-time setup for the CADCraft plugin. Installs the cadcraft-cli program that the plugin's MCP server runs. Run it when the CADCraft tools are missing or the server fails to start.
disable-model-invocation: true
---

# CADCraft setup

Install `cadcraft-cli`, the program behind the CADCraft MCP server. The installer downloads a
prebuilt binary from the project's GitHub release (a few seconds) and checks its SHA-256. If there is
no binary for this system, or the download fails, it builds from source with Rust's `cargo`
(about 5 minutes).

1. Run the installer with the Bash tool. Give it a generous timeout (at least 10 minutes), because the source-build fallback compiles the whole project:

   ```bash
   bash "${CLAUDE_PLUGIN_ROOT}/scripts/install-cadcraft-cli.sh"
   ```

2. Read the result:
   - **Success:** it prints `Installed: <path>` and the version. Tell the user to run `/reload-plugins` (or restart Claude) so the `cadcraft` MCP server starts, and that the CADCraft tools will then be available. Mention whether it was the prebuilt binary or a source build.
   - **Exit code 2 (no prebuilt binary and no usable Rust):** the script prints what to do. Relay it plainly: install Rust from https://rustup.rs (or run `rustup update stable`), open a new terminal, then run `/cadcraft:setup` again. Do not install Rust yourself.
   - **A checksum mismatch:** the script refuses the download and falls back. Report it to the user as it is; do not work around it.
   - **Any other failure:** show the last lines of the error and stop. Common causes are no network access to github.com and, for source builds, a missing C compiler or linker (on Windows, the Visual Studio Build Tools).

3. If the user already has a `cadcraft-cli` binary, no install is needed. They can put it on their `PATH`, or set `CADCRAFT_CLI=/full/path/to/cadcraft-cli`, then run `/reload-plugins`.

To update later, run the same script with `--force`. To skip the download and always build from source, add `--from-source`.
