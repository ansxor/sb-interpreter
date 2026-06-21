---
title: CLS
slug: docs-ptc-cls
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-cls
content_id: 19557
created: 2023-03-15
scraped: 2026-06-21
---

# CLS

Clear the upper and lower text screens.
## Syntax
```sbsyntax
CLS
```
## Examples
```sb
CLS
```
## Notes
`CLS` clears the contents of both text screens - text set by `PRINT` and by `PNLSTR` is cleared. This sets all characters to the zero character `CHR$(0)`. `CLS` does not reset the text foreground or background color, however. Executing `CLS` will fill the screen with the currently selected color and background color, and future text printed will maintain the last color set. The text cursor is reset to the top left of the screen (`CSRX=0`, `CSRY=0`). 

## See Also
- [Console overview](https://smilebasicsource.com/forum/thread/docs-ptc-console)
- [`ACLS`](https://smilebasicsource.com/forum/thread/docs-ptc-acls)
