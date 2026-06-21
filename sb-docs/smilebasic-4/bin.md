---
title: BIN$
slug: docs-sb4-bin
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-bin
content_id: 19492
created: 2020-10-28
scraped: 2026-06-21
---

# BIN$

Convert an integer to a binary string.

## Syntax

```sbsyntax
BIN$ number% {, digits% } OUT string$
```

| Input | Description |
| --- | --- |
| `number%` | The number to convert to binary. |
| `digits%` | The minimum number of digits to return (0-32). (Optional, default 0.) |

| Output | Description |
| --- | --- |
| `string$` | The binary representation of `number%`. |

If `digits%` is omitted or less than the minimum number of digits required to represent `number%` in binary, the returned string will include exactly as many as necessary. If it is greater, then the binary string will be padded with `0` to the specified number of digits.

## Examples

```sb4
'display a binary number
PRINT BIN$(79)
```

```sb4
'display a binary number, with leading zeros
PRINT BIN$(79,32)
```
