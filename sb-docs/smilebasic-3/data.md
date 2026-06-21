---
title: DATA
slug: docs-sb3-data
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# DATA

> **Category:** Basic instructions (data operations and others)

Defines data to read with READ

- Numerical values and character strings can be mixed
- Expressions containing only numerical constants are handled as constants, and so can be written in DATA

statements

- Constants starting with # are also allowed
- Expressions where &&, ||, variables, and functions are mixed are not allowed
- Character string expressions are not allowed

## Format

```sb3
DATA Data [, Data
…
]
```

## Notation of Data

```
- List numerical values and character strings, separating each one with ','
- Character strings must be enclosed in double quotations ("") ("" cannot be omitted)
```

## Examples

```sb3
READ X,Y,Z,ST$ 'Comments can be written
DATA 123,345,56,"SAMPLE"
```
