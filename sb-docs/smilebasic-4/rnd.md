---
title: RND
slug: docs-sb4-rnd
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-rnd
content_id: 19500
created: 2021-01-14
scraped: 2026-06-21
---

# RND

Generate a random integer. The number will be greater than or equal to 0 and less than the specified maximum; or `0 <= number < maximum`.

## Syntax

```sbsyntax
RND { genID%, } maximum% OUT num%
```

| Input | Description |
| --- | --- |
| `genID%` | ID of the generator to use, 0-7. Optional, 0 if omitted. |
| `maximum%` | The upper limit of numbers to generate. |

| Output | Description |
| --- | --- |
| `num%` | The generated number. `0 <= num% < maximum%` |

## Examples

```sb4
' generate a random number from 0..9
PRINT RND(10)
```

```sb4
'use different generators and seeds
RANDOMIZE 0,10
RANDOMIZE 1,20
PRINT RND(0,10)
PRINT RND(1,10)
```
