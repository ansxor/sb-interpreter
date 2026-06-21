---
title: LOAD
slug: docs-sb4-load
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-load
content_id: 19457
created: 2020-10-26
scraped: 2026-06-21
---

# LOAD

Load a program, stored in a TXT file, to a program slot.

## Syntax

```sbsyntax
LOAD file$ {, slot% }
```

| Input | Description |
| --- | --- |
| `file$` | The name or path of the file to load. |
| `slot%` | The program slot used (optional; default 0.) |

The `file$` parameter may optionally start with `TXT:`; e.g. `"TXT:FOO"` and `"FOO"` refer to the same file. Files from other projects can be loaded by specifying the project path; e.g. `"BAR/FOO"` or `"/FOO"`

## Examples

```sb4
'load FOO into slot 0
LOAD "FOO"
LOAD "TXT:FOO"
LOAD "FOO",0
```

```sb4
'load FOO from the project BAR
LOAD "BAR/FOO"
```
