---
title: PROJECT
slug: docs-sb4-project
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-project
content_id: 19455
created: 2020-04-29
scraped: 2026-06-21
---

# PROJECT

Set or get the active project; working with files will default to this project. *Setting the active project can only be used in Direct Mode.*

## Syntax

```sb4
PROJECT name$
active$ = PROJECT()
PROJECT OUT active$
```

| Input | Description |
| --- | --- |
| `name$` | Name of the project to use |

| Output | Description |
| --- | --- |
| `active$` | Name of the current active project |

## Examples

```sb4
' set the active project (Direct only)
PROJECT "FOO"
' check the active project
PRINT PROJECT() '> FOO
```
