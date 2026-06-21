---
title: BGMPLAY
slug: docs-sb3-bgmplay
system: SmileBASIC 3
type: command
category: Sound
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGMPLAY

> **Category:** Sound

## BGMPLAY (1)

Plays music (Plays back registered BGM)

- Up to 8 tunes can be played simultaneously (The total maximum number of sounds that can be generated

simultaneously is 16)

- See the second page for information on how to play music using MML

### Format

```sb3
BGMPLAY [Track number,] Tune number [,Volume]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Track number` | Track number to play back: 0-7 (If omitted, number 0) |
| `Tune number` | - Preset tune: 0-42<br>- User-defined tune: 128-255<br>- A list of preset sounds can be viewed by pressing the SMILE button. |
| `Volume` | Volume level for playback: 0-127 |

### Examples

```sb3
BGMPLAY 0
```

## BGMPLAY (2)

Plays music (Plays back the input MML data)

- MML playback is performed in track 0
- The MML tune will overwrite user-defined tune number 255
- Executing immediately after BGMPLAY will cause a delay of approx. 2 frames

### Format

```sb3
BGMPLAY "MML string"
```

### Arguments

| Argument | Description |
| --- | --- |
| `MML string` | - Pressing the Help button for "MML" will display descriptions of MML commands<br>- You can register a character string to play by listing the following symbols:<br>:0 - :15  Channel specification<br>T1 - T512 Tempo specification<br>CDEFGAB   Scale (C# is a halftone higher; C- is a halftone lower)<br>N0 - N127 Key value specification (O4C=60)<br>1 - 192   Individual tone length specification (C1 = Whole note, C4. = Dotted quarter note)<br>L1 - L192 Default tone length (. should be used for dotted notes)<br>R Rest<br>O0 - O8   Octave number specification<br>< ><br>One octave up or down<br>V0 - V127 Volume level value specification<br>( )<br>Volume up or down<br>@0 - @255 Tone change (0 - 127: Equivalent to GM, 224-: User-defined waveform)<br>P0 - P127 Pan pot (Left: P0-63 Center: P64 Right: P65-127)<br>[<br>Repeat start<br>]<br>Number of times Repeat end (If the number of times is omitted, the loop will be<br>endless)<br>&<br>Connects the preceding/succeeding notes<br>_<br>Portamento |

### Examples

```sb3
BGMPLAY "T120O4L4CC8D8EE8F8GA8G8E2"
```
