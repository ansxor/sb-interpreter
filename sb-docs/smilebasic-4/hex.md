---
title: HEX$
slug: docs-sb4-hex
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-hex
content_id: 19491
created: 2020-10-28
scraped: 2026-06-21
---

# HEX$

Convert an integer to a hexadecimal string.

## Syntax

```sbsyntax
HEX$ number% {, digits% } OUT string$
```

| Input | Description |
| --- | --- |
| `number%` | The number to convert to hex. |
| `digits%` | The minimum number of digits to return (0-8). (Optional, default 0.) |

| Output | Description |
| --- | --- |
| `string$` | The hexadecimal reprecentation of `number%`. |

If `digits%` is omitted or less than the minimum number of digits required to represent `number%` in hex, the returned string will include exactly as many as necessary. If it is greater, then the hex string will be padded with `0` to the specified number of digits.

## Examples

```sb4
'display a hex number
PRINT HEX$(79)
```

```sb4
'display a hex number, with leading zeros
PRINT HEX$(79,8)
```
