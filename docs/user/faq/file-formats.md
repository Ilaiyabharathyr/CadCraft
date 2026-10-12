---
title: Which file formats does CADCraft support?
description: CADCraft opens and saves DXF and DWG drawings and exports PDF, SVG and PNG.
order: 10
verified: 0.5.0
---

CADCraft opens and saves DXF and DWG. It exports PDF, SVG and PNG.

| Format | Open                               | Save                               |
|--------|------------------------------------|------------------------------------|
| DXF    | R12 through 2018, ASCII and binary | 2000 through 2018, 2018 by default |
| DWG    | R13 through 2018                   | 2000 through 2018, 2018 by default |

Export formats are PDF, SVG and PNG, from **File ▸ Export...**, **File ▸ Export to PDF...** or **File ▸ Print...**.

Not everything in a drawing survives yet. CADCraft keeps an object it does not model as raw data in a DXF file, but a DWG save drops some of them, such as 3D solids and surfaces. Keep the original file, and see [Open and save DXF and DWG drawings](../guides/open-and-save-drawings.md).

CADCraft draws in 2D only. Commands for 3D orbiting report that they are not available yet.
