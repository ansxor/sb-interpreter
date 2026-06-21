---
title: INKEY$
slug: docs-sb4-inkey
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-inkey
content_id: 19522
created: 2021-12-02
scraped: 2026-06-21
---

# INKEY$

Pop a character off the keyboard input buffer. This buffer is used by the USB and software keyboard, as well as `PUSHKEY`. This is the primary way of reading text aside from `INPUT` and `LINPUT`. If the buffer is empty, an empty string is returned.

Some controller buttons push special characters into the buffer. A flag can be passed to ignore them.

## Syntax

```sb4
char$ = INKEY$({ blockController% })
INKEY$ { blockController% } OUT char$
```

| Input | Description |
| --- | --- |
| `blockController%` | Ignore special characters pushed by controller inputs. |

| Output | Description |
| --- | --- |
| `char$` | A string containing a single character, or the empty string if the keyboard buffer is empty. |

## Controller Input Characters

Pressing certain buttons on any controller will push characters to the keyboard buffer. These characters are equivalent to specific keyboard keys as well. If the `blockController%` flag is passed, characters pushed by controllers will be blocked.

| Button | Key | Character Code |
| --- | --- | --- |
| `#B_LRIGHT` D-Pad Right | Arrow Key Right | `001C` |
| `#B_LLEFT` D-Pad Left | Arrow Key Left | `001D` |
| `#B_LUP` D-Pad Up | Arrow Key Up | `001E` |
| `#B_LDOWN` D-Pad Down | Arrow Key Down | `001F` |
| `#B_A` A Button | Enter | `000D` |
| `#B_Y` Y Button | Backspace | `0008` |

## Examples

```sb4
ACLS
LOOP
 VSYNC
 VAR CHAR$ = INKEY$()
 IF CHAR$ != "" THEN PRINT CHAR$, ASC(CHAR$)
ENDLOOP
```
