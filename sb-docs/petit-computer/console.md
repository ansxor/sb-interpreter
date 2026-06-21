---
title: Console overview
slug: docs-ptc-console
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-console
content_id: 19561
created: 2023-03-18
scraped: 2026-06-21
---

# Console overview

## Summary
The text console provides a simple method for input and output via text, using `INPUT`/`LINPUT` and `PRINT`. The console also provides basic formatting options such as changing the location of printed text and setting colors/backgrounds for each character. The console also automatically scrolls if too much text is printed.

## Interface
### Commands
| [`ACLS`](https://smilebasicsource.com/forum/thread/docs-ptc-acls) | Clears the text console and several other things |
| --- | --- |
| [`CLS`](https://smilebasicsource.com/forum/thread/docs-ptc-cls) | Clears the text console, as well as lower screen text. |
| [`LOCATE`](https://smilebasicsource.com/forum/thread/docs-ptc-locate) | Sets the current text cursor position.|
| [`COLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-color) | Sets the foreground color palette and background color for text. |
| [`PRINT`](https://smilebasicsource.com/forum/thread/docs-ptc-print) | Prints text to the screen.|
| [`INPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-input) | Gets user input and stores the result into one or more variables.|
| [`LINPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-linput) | Gets user input and stores the result into a variable.|

### Functions
[`CHKCHR`](https://smilebasicsource.com/forum/thread/docs-ptc-chkchr) — Read a character from the console.

### System Variables
| `CSRX` | The current X location of the cursor |
| --- | --- |
| `CSRY` | The current Y location of the cursor |
| `TABSTEP` | The current number of spaces in one tab |

### Resources
| `BGF0U` | This is the font resource used. |
| --- | --- |
| `BGD0U` | This is used for `COLOR` backgrounds |
| `SPS1U` | This contains the text cursors used for `INPUT` and `LINPUT` |

## Additional Information
The text console itself is rendered in the same way as the background layers are, as two 64x64 tilemaps with 4-bit color 8x8 pixel tiles. The foreground layer is used for the text characters, and the background layer is used for the `COLOR` background setting.

Unlike the background layers, the text console cannot be moved, so only the upper-left 32x24 tile region visible is used.

The text layer uses `BGF` resources; the background layer uses `BGD` resources. It is possible to print characters not in `BGF0` by loading a `MEM` containing those characters, which allows you to access a sort of "`BGF1`" containing the editor line-number characters and some other extras.

The text background layer uses either `BGD0U` tile 0 (transparent, used for background color 0) or tile 15 (solid tile of color 15 of background palette). It is possible to redefine these tiles using `LOAD` or `CHRSET` to change the background tile used.

While it is possible to read the normal set of `CHR$(0)` to `CHR$(255)` characters using `CHKCHR`, it is not possible to distinguish the "extra" characters such as the line-number characters - the codes returned will still be in the 0-255 range. It is not possible to read the foreground or background colors from the console, or the currently set foreground or background color.
