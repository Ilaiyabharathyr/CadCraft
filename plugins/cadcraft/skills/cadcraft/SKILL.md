---
name: cadcraft
description: Draw, edit, inspect and export 2D CAD drawings with CADCraft (an AutoCAD-style drafting app) through its MCP tools. Use when the user wants floor plans, mechanical part drawings, dimensions, hatches, layers or blocks, or wants to read, convert or render a DXF/DWG file to PNG, SVG or PDF.
---

# CADCraft: CAD drafting from Claude

CADCraft is an open-source, AutoCAD-style 2D/3D drafting application. This plugin runs its headless
MCP server (`cadcraft-cli mcp`), so there is no window: you draw by typing commands, check the
result with inspection tools, and look at it with `render`.

## Before you start

The MCP tools below come from the `cadcraft` server. If they are not available, `cadcraft-cli` is
probably not installed yet. Tell the user to run `/cadcraft:setup` (a one-time build, about 5
minutes), then `/reload-plugins`. Do not try to install it yourself.

## Tools

| Tool | Use it to |
|---|---|
| `new_drawing {metric?}` | Start a drawing. Pass `metric: true` for millimetres; the default is imperial. |
| `command_line {text}` | Type at the command line like a person: `line 0,0 @10,0 @0,5 c`. Spaces act as Enter. |
| `script {text}` | Run several lines at once, like an AutoCAD `.scr` file. |
| `execute {command, params}` | Run one command with JSON parameters. Never opens a dialog. Best for exact geometry. |
| `list_commands {filter?}` | Find command ids, aliases and parameter docs (about 400 commands). |
| `inspect_drawing {entities?, limit?}` | Counts per type, extents, layers, styles, blocks, layouts, selection, undo history. |
| `query_entities {type?, layer?, window?, crossing?, limit?, offset?}` | Entities with handles and geometry. |
| `find_text {contains?, layer?, window?}` | Read TEXT, MTEXT, attributes and leader notes (room names, titles, notes). |
| `render {width?, height?, window?}` | A PNG of the drawing, or of just one area. This is how you look at your work. |
| `open {path}` / `save {path?}` | Open a DXF/DWG. Save by extension: `.dxf`, `.dwg`, or export to `.svg`, `.png`, `.pdf`. |
| `cancel` | Press Escape: abandon a half-finished command. |

## Workflow

1. `new_drawing` (set `metric` to match the user's units), or `open` an existing file.
2. Set up layers first with `execute layer.new {name, color?, linetype?, current?}`, then draw on the right layer.
3. Draw and modify. Use `execute` for exact numbers and `command_line` for AutoCAD-style sequences.
4. **Verify every meaningful step.** `inspect_drawing` for counts and layers, `query_entities` for geometry, and `render` to see it. Never assume a command worked because it returned no error.
5. Deliver with `save`: `.dxf` or `.dwg` for CAD software, `.pdf` or `.svg` for sharing, `.png` for a preview.

## Syntax that works

Coordinates are `x,y` with y up. `@dx,dy` is relative, `@dist<angle` is polar (angle in degrees), and an empty
line is Enter.

```text
command_line  rectang 0,0 100,60              rectangle from two corners
command_line  line 0,0 @10,0 @0,5 c          closed triangle (c = Close)
command_line  circle 50,30 10                 centre and radius
command_line  dimlinear 0,0 100,0 50,-8       dimension: two points, then the line position
command_line  zoom e                          zoom to extents
```

```json
execute  line     {"points": [[0,0],[10,0]]}
execute  circle   {"center": [50,30], "radius": 10}
execute  rectang  {"p1": [0,0], "p2": [100,60]}
execute  text     {"at": [5,52], "text": "ROOM A", "height": 4}
execute  hatch    {"points": [[2,2]], "pattern": "ANSI31", "scale": 2}
execute  layer.new {"name": "Walls", "color": "red", "current": true}
execute  zoom     {"mode": "extents"}
execute  offset   {"handle": "101", "distance": 2, "side": [60,30]}
```

A `hatch` given an internal point fills the closed boundary around it and leaves islands (such as a
circle inside a rectangle) unfilled. Entities are referred to by hex `handle`, which every create
command returns and `query_entities` lists.

The exact parameters of any command are in `list_commands {filter: "name"}` (the `params` field). When
`execute` rejects a call, the error says which parameter is missing or wrong; fix it and retry.

## Reading the command line

`command_line` returns the current `prompt` (for example `LINE Specify next point or [Undo]:`),
the accepted `keywords`, and `running` (the active command, or `null` when idle). If a sequence
leaves a command waiting, either finish it or call `cancel`. A line the command line refuses comes
back with an `error` field and does not crash the session.

For a large drawing, do not read everything. Use `inspect_drawing` for the overview, `find_text` to
locate rooms and notes, `query_entities` with a `window` for one area, and `render` with a `window`
to look at just that area.

## Cautions

- CADCraft is in early development. Entities it does not model yet are **not kept when it saves**. When editing a drawing the user
  already has, save to a new filename and leave the original untouched; tell the user if the entity counts changed.
- The headless server has one drawing session that lives as long as the server. Nothing is written to disk
  until you `save`.
- `screenshot`, `ui_inspect` and `ui_click` exist only when connected to the graphical app, which this plugin does not do.
- Do not claim a drawing is correct from commands alone. Show the user the `render` and say what you verified.
