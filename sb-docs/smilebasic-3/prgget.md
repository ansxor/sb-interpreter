---
title: PRGGET$
slug: docs-sb3-prgget
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGGET$

> **Category:** Source code manipulation

Gets the current single line as a character string

## Format

```sb3
String variable=PRGGET$()
```

## Return Values

```
Source character string of the current line (or an empty string if there is no applicable line)
```

## Examples

```sb3
A$=PRGGET$()
```
