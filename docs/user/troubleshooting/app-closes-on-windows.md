---
title: The app closes right after opening on Windows
description: Fix CADCraft 0.5.0 on Windows 11 closing a moment after its window appears, by starting it with the Vulkan graphics backend.
order: 10
verified: 0.5.0
---

If CADCraft 0.5.0 opens a window and closes again within a second, start it with the Vulkan graphics backend from PowerShell. That works around the problem. On Intel graphics, use `gl` instead.

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

On Intel graphics (UHD, Iris), use `gl` in place of `vulkan`. Intel's Vulkan driver (`igvk64.dll`) has crashed CADCraft at start ([issue #33](https://github.com/storytold/cadcraft/issues/33)).

On AMD graphics do not use `gl`: it can bring back the `atio6axx.dll` driver crash.

## Why this happens

CADCraft 0.5.0 draws with DirectX 12 on Windows and compiles its shaders with DXC. The 0.5.0 Windows build does not ship the DXC compiler files (`dxcompiler.dll` and `dxil.dll`), so on some PCs the graphics device is lost before the first frame and the app exits. This is issue [#608](https://github.com/storytold/cadcraft/issues/608). The issue reports that starting with Vulkan instead gets the app running. A fix that uses FXC, the compiler built into Windows, was merged after 0.5.0 ([pull request #613](https://github.com/storytold/cadcraft/pull/613)) and is not in a release yet.

## Still stuck?

Follow [Report a problem](index.md#report-a-problem) and attach `cadcraft.log`. Mention your graphics card.
