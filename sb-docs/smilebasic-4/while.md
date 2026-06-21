---
title: WHILE ~ WEND
slug: docs-sb4-while
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-while
content_id: 19508
created: 2020-11-04
scraped: 2026-06-21
---

# WHILE ~ WEND

A `WHILE` block creates a loop that runs *while* a condition is true. The condition is evaluated at each iteration, so if it changes to false, the loop will exit.

## Syntax

```sb4
WHILE condition
 <body>
WEND
```

| Name | Description |
| --- | --- |
| `WHILE` | The `WHILE` keyword is the start of a `WHILE` block. |
| `condition` | An expression that results in a true or false value. |
| `<body>` | Code within the `WHILE` block is executed at each iteration of the loop. |
| `WEND` | The `WEND` keyword is the end of a `WHILE` block. |

## Examples

The `WHILE` loop is used to repeat a block of code while some trivial condition remains true. The condition must have some way to change within the loop, otherwise the loop will continue forever (unless a statement within contains `BREAK` or some other means of escape.)
