---
title: LOOP ~ ENDLOOP
slug: docs-sb4-loop
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-loop
content_id: 19510
created: 2020-11-04
scraped: 2026-06-21
---

# LOOP ~ ENDLOOP

A `LOOP` block loops a section of code forever.

## Syntax

```sb4
LOOP
 statements...
ENDLOOP
```

| Name | Description |
| --- | --- |
| `statements...` | Statements inside the `LOOP` block will be repeated. |

## Examples

Anything inside the `LOOP` will be looped forever, unless there is a way for the loop to exit (e.g. a `BREAK` statement.)

```sb4
'this loop runs forever
LOOP
 PRINT "I'm running forever!"
ENDLOOP
```

```sb4
'this loop will exit if the random number is 7
'or, each iteration has a 1/10 chance of exiting
LOOP
 PRINT "Will I escape?"
 IF RND(10)==7 THEN BREAK
ENDLOOP
PRINT "I made it!"
```

Most complex interactive programs have a "main loop" that contains the core logic. It's a common-sense design pattern to have a block of code that coordinates the rest of your program, and it only makes sense to have it loop as long as necessary. `LOOP` is the perfect candidate.

```sb4
'a simple interactive program
LOOP
 VSYNC
 IF BUTTON(0,#B_A) THEN PRINT "A button"
 IF BUTTON(0,#B_B) THEN PRINT "B button"
 IF BUTTON(0,#B_Y) THEN CLS
 IF BUTTON(0,#B_X) THEN BREAK
ENDLOOP
```
