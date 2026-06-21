---
title: SNDSTOP
slug: docs-sb4-sndstop
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-sndstop
content_id: 19564
created: 2023-03-21
scraped: 2026-06-21
---

# SNDSTOP

Stop all audio playing.

## Syntax

```
SNDSTOP
```

## Examples

```
'play a song
BGMPLAY 0
'play lots of sound effects
FOR I%=1 TO 10
 BEEP RND(10)
NEXT I%
'stop (after waiting so you can hear it for the example :))
WAIT 5
SNDSTOP
```
