---
title: Graphics overview
slug: docs-ptc-graphics
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-graphics
content_id: 19742
created: 2024-11-30
scraped: 2026-06-21
---

# Graphics overview

## Summary

There are four GRP pages available for use, each 256 pixels wide by 192 pixels tall, with a color palette capable of displaying 256 unique colors. Each screen can be set to use one of the four pages - both can be the same page, or different ones.

## Interface

### Commands

| | |
| --- | --- |
| [`GPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-gpage) | Select the current screen to modify. Also, can change the displayed GRP page. |
| [`GPRIO`](https://smilebasicsource.com/forum/thread/docs-ptc-gprio) | Sets the display priority of the GRP page. |
| [`GCOLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-gcolor) | Changes the default color used for graphics commands. |
| [`GCLS`](https://smilebasicsource.com/forum/thread/docs-ptc-gcls) | Clears the selected page. |
| [`GPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-gpset) | Set a single pixel on the selected page. |
| [`GLINE`](https://smilebasicsource.com/forum/thread/docs-ptc-gline) | Draw a line on the selected page. |
| [`GBOX`](https://smilebasicsource.com/forum/thread/docs-ptc-gbox) | Draw a hollow rectangle on the selected page. |
| [`GFILL`](https://smilebasicsource.com/forum/thread/docs-ptc-gfill) | Draw a filled rectangle on the selected page. |
| [`GCIRCLE`](https://smilebasicsource.com/forum/thread/docs-ptc-gcircle) | Draw a circle on the selected page. |
| [`GCOPY`](https://smilebasicsource.com/forum/thread/docs-ptc-gcopy) | Copies a region of a GRP page to the currently selected page. |
| [`GDRAWMD`](https://smilebasicsource.com/forum/thread/docs-ptc-gdrawmd) | Sets the drawing mode. |
| [`GPUTCHR`](https://smilebasicsource.com/forum/thread/docs-ptc-gputchr) | Draw a single CHR character to the selected page. |
| [`GPAINT`](https://smilebasicsource.com/forum/thread/docs-ptc-gpaint) | Flood fills a region of the selected page. |

### Functions

| | |
| --- | --- |
| [`GSPOIT`](https://smilebasicsource.com/forum/thread/docs-ptc-gspoit) | Reads a pixel's color from a GRP page. |

### Resources

| | |
| --- | --- |
| `GRP0`-`GRP3` | Graphics page resources. These contain the pixel data, but not the colors. |
| `COL2U` | Color palette used for upper screen graphics page. |
| `COL2L` | Color palette used for lower screen graphics page. |

## Additional Information

Note that the graphics pages are shared between both screens, but which are displayed can be changed. The default display uses `GRP0` for the upper screen and `GRP1` for the lower screen.

The graphics pages can not be moved, and always fill the screen they are on.

The GRP pixel data is broken up into character units and sprite units. A character unit is an 8x8 region of pixels, and a sprite unit is a 64x64 region of pixels composed of those 8x8 character units. The full page is composed of 12 sprite units, ordered from left-to-right and then top-to-bottom.
