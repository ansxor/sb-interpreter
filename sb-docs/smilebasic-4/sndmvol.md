---
title: SNDMVOL
slug: docs-sb4-sndmvol
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-sndmvol
content_id: 19565
created: 2023-03-21
scraped: 2026-06-21
---

# SNDMVOL

Set or get the master volume of all audio sources.

## Syntax

```sb4
SNDMVOL volume% {, fadeTime# }
volume% = SNDMVOL()
SNDMVOL OUT volume%
```

| Parameter  | Description |
| --- | --- |
| `volume%`   | The master volume level. 0-127. |
| `fadeTime#` | Time (in seconds) spent fading to the set volume. 0-32767. Optional, default 0. |

## Examples

```sb4
'change the volume
SNDMVOL 64
```

```sb4
'fade all sound out for 2 seconds
SNDMVOL 0,2
```
