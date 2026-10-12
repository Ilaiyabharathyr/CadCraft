---
title: Can I script CADCraft or use it with an AI assistant?
description: CADCraft has an MCP server, a JSON control channel and a command-line tool, documented for developers on GitHub.
order: 30
verified: 0.5.0
---

Yes. Every menu item, tool and prompt in CADCraft is a command, and the same commands can be driven from outside the window.

- **MCP server**, so an AI assistant can work in a drawing. See the [MCP guide](https://github.com/storytold/cadcraft/blob/main/docs/mcp.md).
- **JSON control channel** for the running desktop app. See the [control protocol](https://github.com/storytold/cadcraft/blob/main/docs/control-protocol.md).
- **`cadcraft-cli`** for headless runs such as `cadcraft-cli info drawing.dxf` and `cadcraft-cli convert drawing.dxf drawing.svg`.

Inside the app, the command line also runs scripts in the style of `.scr` files.

These pages are for developers, and they are not part of this documentation.
