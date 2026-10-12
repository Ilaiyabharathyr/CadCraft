---
title: Open and save DXF and DWG drawings
description: Open DXF and DWG files in CADCraft, save them in either format, and pick the default format and version.
order: 20
verified: 0.5.0
---

CADCraft opens DXF and DWG drawings and saves both. New drawings are saved as DXF unless you change the default.

## Open a drawing

1. Choose **File ▸ Open...**.
2. In the file dialog, pick a `.dwg`, `.dxf` or `.dwt` file. The first file type in the list is **Drawings (DWG, DXF)**.
3. Click Open. The drawing opens.

CADCraft reads DXF from R12 through 2018, ASCII and binary, and DWG from R13 through 2018.

## Save a drawing

1. Choose **File ▸ Save** to save over the file you opened. A new drawing has no file yet, so this asks where to save.
2. To save under a new name or in another format, choose **File ▸ Save As...**.
3. In the dialog, pick the file type. The choices are **Drawing (DXF)**, **Drawing (DWG)**, **Drawing Template (DWT)**, **SVG**, **PNG** and **PDF**. A name ending in `.dwg` or `.dxf` selects that type first.

A drawing you opened from a DWG file is saved to a file of the same type by default.

## Choose the default format and version

1. Choose **File ▸ Default Save Format**.
2. Pick **DXF** or **DWG** for new drawings.
3. Pick a version for new drawings: **Version 2018**, **Version 2013**, **Version 2010**, **Version 2007**, **Version 2004** or **Version 2000**. Older versions leave out what the old format cannot hold, such as true colors.

CADCraft saves DXF and DWG at 2000 through 2018, and 2018 by default.

## What to know before you trade files with other CAD programs

CADCraft does not keep every kind of object yet. Some 3D objects, such as 3DSOLID, BODY, SURFACE and MESH, are kept in a DXF file as they were read but are dropped when you save as DWG. The [file format notes](https://github.com/storytold/cadcraft/blob/main/docs/file-format-parity.md) list what is kept and what is lost. Keep a copy of the original file until you have checked the saved one.

## Related

- [Export a drawing to PDF, SVG or PNG](export-pdf-svg-png.md)
- [FAQ: Which file formats does CADCraft support?](../faq/file-formats.md)
