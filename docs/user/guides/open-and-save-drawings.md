---
title: Open and save DXF and DWG drawings
description: Open DXF and DWG files in CADCraft, save them in either format, and pick the default format for new drawings.
order: 20
verified: 0.5.0
---

CADCraft opens DXF and DWG drawings and saves both. New drawings are saved as DXF unless you change the default.

## Open a drawing

1. Choose **File ▸ Open...**. The shortcut is Ctrl+O.
2. In the file dialog, pick a `.dwg` or `.dxf` file. The first file type in the list is **Drawings (DWG, DXF)**.
3. Click Open. The drawing opens.

CADCraft reads DXF from R12 through 2018, ASCII and binary, and DWG from R13 through 2018.

## Save a drawing

1. Choose **File ▸ Save** to save over the file you opened. The shortcut is Ctrl+S. A new drawing has no file yet, so this asks where to save.
2. To save under a new name or in another format, choose **File ▸ Save As...**. The shortcut is Ctrl+Shift+S.
3. In the dialog, pick the file type: **Drawing (DXF)** or **Drawing (DWG)**. A name ending in `.dwg` or `.dxf` selects that type first.

A drawing you opened from a DWG file is saved to a file of the same type by default.

CADCraft 0.5.0 saves DXF and DWG as R2000. DXF files are saved as ASCII.

## Choose the default format for new drawings

1. Choose **File ▸ Default Save Format**.
2. Pick **DXF** or **DWG**.

## What to know before you trade files with other CAD programs

CADCraft does not keep every kind of object yet. Entities it does not model yet are not kept when you save. The [file format notes](https://github.com/storytold/cadcraft/blob/v0.5.0/docs/file-format-parity.md) list what is kept and what is lost. Keep a copy of the original file until you have checked the saved one.

## Related

- [Export a drawing to PDF, SVG or PNG](export-pdf-svg-png.md)
- [FAQ: Which file formats does CADCraft support?](../faq/file-formats.md)
