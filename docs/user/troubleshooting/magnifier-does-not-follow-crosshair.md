---
title: A screen magnifier does not follow the crosshair
description: Turn on Show System Cursor so Windows Magnifier and other screen tools can follow the pointer over the CADCraft drawing area.
order: 20
verified: 0.5.0
---

Turn on **View ▸ Accessibility ▸ Show System Cursor**. CADCraft then keeps a small system crosshair at the center of its own crosshair, and screen magnifiers can follow it.

## Symptoms

- Windows Magnifier is set to follow or center the mouse pointer, but the view stays still over the drawing area.
- A screen recorder or remote desktop tool does not show the pointer over the drawing area.

## Fix

1. Choose **View ▸ Accessibility ▸ Show System Cursor**.
2. Move the pointer over the drawing area. A small system crosshair is visible at the center of the drawn one.

To have it on at every start, set the environment variable `CADCRAFT_SYSTEM_CURSOR=1`. On Windows, run `setx CADCRAFT_SYSTEM_CURSOR 1` once and start CADCraft again.

## Why this happens

Over the drawing area CADCraft hides the system cursor and draws its own crosshair. Tools that follow the system cursor lose track of it there. The behavior is tracked in issue [#39](https://github.com/storytold/cadcraft/issues/39).

## Still stuck?

Follow [Report a problem](index.md#report-a-problem).
