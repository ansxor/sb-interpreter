---
title: METASAVE
slug: docs-sb4-metasave
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-metasave
content_id: 19459
created: 2020-10-26
scraped: 2026-06-21
---

# METASAVE

Save the metadata of the active project.

## Syntax

```sbsyntax
METASAVE
```

The metadata set by `METAEDIT` will be saved to the active project.

## Examples

```sb4
'change the active project's title and save
METAEDIT 0,"TITLE"
METASAVE
```
