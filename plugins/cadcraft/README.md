# CADCraft plugin for Claude

Lets Claude draw, dimension, hatch, inspect and export 2D CAD drawings with
[CADCraft](https://github.com/ihayashi/cadcraft), an open-source, AutoCAD-style drafting
application written in Rust.

The plugin starts CADCraft's own headless MCP server (`cadcraft-cli mcp`) and adds a skill that
teaches Claude the command syntax and a draw, verify, render, save workflow. There is no window:
Claude looks at the drawing through rendered PNG images, and saves DXF, DWG, SVG, PNG or PDF files.

## Install

You need [Claude Code](https://code.claude.com). Rust is only needed if there is no prebuilt binary for your system.

```text
/plugin marketplace add ihayashi/cadcraft
/plugin install cadcraft@cadcraft-plugins
/cadcraft:setup
/reload-plugins
```

`/cadcraft:setup` is a one-time step. It downloads a prebuilt `cadcraft-cli` for your system from
the project's [GitHub releases](https://github.com/ihayashi/cadcraft/releases) into `~/.cadcraft`
(a few seconds), after checking it against the release's `SHA256SUMS.txt`. If there is no binary for
your system, or the download fails, it falls back to building from source with
`cargo install --git https://github.com/ihayashi/cadcraft --locked cadcraft-cli`
(about 5 minutes, needs [Rust](https://rustup.rs) 1.90 or newer). To use a binary you already have,
put `cadcraft-cli` on your `PATH` or set `CADCRAFT_CLI=/full/path/to/cadcraft-cli` and skip the setup.

Prebuilt binaries: Linux (x86_64, aarch64), macOS (Apple Silicon, Intel) and Windows (x86_64).

Supported on macOS and Linux. On Windows it works through Git Bash (which Claude Code uses), and
a source build needs the Visual Studio Build Tools.

## Use

Just ask. For example:

- "Draw a 100 x 60 mm plate with a 20 mm hole in the centre, dimension it, and show me."
- "Open `plan.dxf`, list the room names, and render the kitchen area."
- "Convert `drawing.dwg` to a PDF."

Claude will build the drawing with the CADCraft tools, check it with `inspect_drawing` and
`render`, and save the result where you ask.

## What is included

| Path | What it is |
|---|---|
| `.mcp.json` | Registers the `cadcraft` MCP server. |
| `scripts/cadcraft-mcp.sh` | Finds `cadcraft-cli` and starts the server; explains how to fix it if missing. |
| `scripts/install-cadcraft-cli.sh` | The one-time installer used by `/cadcraft:setup`: download, or build from source. |
| `skills/cadcraft/` | How Claude should draft with CADCraft. |
| `skills/setup/` | The `/cadcraft:setup` command. |

## Configuration

| Variable | Effect |
|---|---|
| `CADCRAFT_CLI` | Full path to a `cadcraft-cli` binary to use instead of searching. |
| `CADCRAFT_HOME` | Install root for the setup script. Default `~/.cadcraft`. |
| `CADCRAFT_DOWNLOAD_BASE` | Where the setup script downloads release files from. Default: the repository's latest release. |
| `CADCRAFT_REPO_URL` | Repository to download from or build from. Default `https://github.com/ihayashi/cadcraft`. |
| `CADCRAFT_GIT_REF` | Branch, tag or commit to build from source, instead of the default branch (skips the download). |

## Publishing the binaries (for maintainers)

`.github/workflows/cadcraft-cli-release.yml` builds `cadcraft-cli` for the five platforms above,
smoke-tests it, and attaches `cadcraft-cli-<target>.tar.gz` plus `SHA256SUMS.txt` to a GitHub
release. Publish one by pushing a tag, or from the Actions tab (**CLI binaries**, Run workflow):

```bash
git tag cli-v0.7.0 && git push origin cli-v0.7.0
```

The setup script downloads from the repository's *latest* release, so publish binaries before
telling people to install, and re-run the workflow after changing the CLI.

## Limits

CADCraft is in early development. Entities it does not model yet are not kept when it saves, so
edit copies of your drawings and compare the entity counts. The plugin runs the headless server
only; it does not drive the graphical app.

## License

The plugin is released under the same terms as CADCraft: MIT OR Apache-2.0. CADCraft is an
independent project and is not affiliated with or endorsed by Autodesk. AutoCAD and DWG are
trademarks of Autodesk, Inc.
