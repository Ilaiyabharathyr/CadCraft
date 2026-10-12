---
title: Export a drawing to PDF, SVG or PNG
description: Save a CADCraft drawing as a PDF with File Export to PDF, or as an SVG or PNG image with File Save As.
order: 30
verified: 0.5.0
---

To get a PDF, choose **File ▸ Export to PDF...**. For an SVG or PNG image, choose **File ▸ Save As...** and pick the SVG or PNG file type.

## Export to PDF

1. Open the drawing.
2. Choose **File ▸ Export to PDF...**. **File ▸ Print...** (Ctrl+P) asks for a PDF file name in the same way.
3. In the dialog, pick a folder and a name. The suggested name is the drawing's name with `.pdf`.
4. Save. The command line reports `Plotted to` followed by the path.

## Export to SVG or PNG

1. Open the drawing.
2. Choose **File ▸ Save As...**. The shortcut is Ctrl+Shift+S.
3. Pick the file type **SVG** or **PNG**, or type a name that ends in `.svg` or `.png`.
4. Save.

An export only writes the image or PDF. The drawing keeps its own name and still counts as unsaved if you had changes, so save it as DXF or DWG too.

## From the command line

`cadcraft-cli convert drawing.dxf drawing.svg` converts a drawing without opening the window.

## Related

- [Open and save DXF and DWG drawings](open-and-save-drawings.md)
