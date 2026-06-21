---
title: BEEP
slug: docs-sb4-beep
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-beep
content_id: 19566
created: 2023-03-21
scraped: 2026-06-21
---

# BEEP

Play a sound effect. A sound effect handle can be returned to control individual sound effects after they start playing.

## Syntax

```sb4
BEEP soundID%, pitch%, volume%, pan%
handle% = BEEP(soundID%, pitch%, volume%, pan%)
BEEP soundID%, pitch%, volume%, pan% OUT handle%
```

| Input | Description |
| --- | --- |
| `soundID%` | Sound sample number to play. Default 0. See sample number table. |
| `pitch%` | Adjusts the pitch of the sound effect. -32768 to 32767.<br>100 = one semitone. Default 0 (no change.) |
| `volume%` | Volume of the sound effect. 0 to 127. Default 64. |
| `pan%` | Stereo pan of the sound effect. 0 (far left) to 127 (far right.) Default 64 (center.) |

| Output | Description |
| --- | --- |
| `handle%` | A handle number used to change sound effect properties while playing. 0 to 15. Optional. |

`BEEP` plays sound samples (either built-in or user-programmed) as a single sound effect. All parameters to `BEEP` are optional, including the `handle%` return value, and all inputs can be left empty. If an input is empty or not specified its default value is used. This allows `BEEP` to be used very flexibly. Up to 16 sound effects can play simultaneously.

The `handle%` returned can be passed to functions such as `BEEPPAN` to change the sound properties after play has started. For long sound effects, you could use this to create interesting effects (left-right panning, warbling pitch, etc.)

## Examples

```sb4
'play the default beep sound
BEEP
```

```sb4
'play an error sound
BEEP 2
```

```sb4
'play a sound at a lower pitch
BEEP 2,-2000
```

```sb4
'play a sound at a low volume
BEEP 2,,16
```

```sb4
'pan a sound effect right
BEEP 2,,,127
```

```sb4
'use all sound properties
BEEP 2,-2000,16,127
```

```sb4
'use a BEEP handle
VAR HANDLE%=BEEP(14)
BEEPPIT HANDLE%,-2000
BEEPVOL HANDLE%,16
BEEPPAN HANDLE%,127
WAIT 5
BEEPSTOP HANDLE%
```
