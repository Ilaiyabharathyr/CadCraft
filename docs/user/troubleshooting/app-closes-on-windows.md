---
title: The app closes right after opening on Windows
description: Fix CADCraft 0.5.0 on Windows 11 closing a moment after its window appears, by starting it with the Vulkan graphics backend.
order: 10
verified: 0.5.0
---

If CADCraft 0.5.0 opens a window and closes again within a second, start it with the Vulkan graphics backend from PowerShell. That works around the problem.

## Symptoms

- The window flashes, or appears for a moment, and the app quits.
- `%APPDATA%\CADCraft\logs\cadcraft.log` contains `Exiting because of error: WGPU error: Parent device is lost`.

## Fix

1. Open PowerShell in the folder that contains `cadcraft.exe`.
2. Run these three lines:

        $env:WGPU_BACKEND = "vulkan"
        & .\cadcraft.exe
        Remove-Item Env:WGPU_BACKEND

The last line clears the setting so later launches use the default again. You need the first two lines each time you start CADCraft this way.

Do not use `gl` in place of `vulkan`: it can bring back a driver crash on some AMD systems.

## Why this happens

CADCraft 0.5.0 draws with DirectX 12 on Windows. On some PCs its shader compiler setup fails before the first frame and the graphics device is lost (issue [#608](https://github.com/storytold/cadcraft/issues/608)). The development version of CADCraft already compiles DirectX 12 shaders with a different compiler. Check the release notes of newer versions.

## Still stuck?

Follow [Report a problem](index.md#report-a-problem) and attach `cadcraft.log`. Mention your graphics card.
