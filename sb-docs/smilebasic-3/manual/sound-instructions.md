---
title: Sound Instructions
slug: docs-sb3-manual-sound-instructions
system: SmileBASIC 3
type: guide
topic: 30
source: e-manual.pdf
scraped: 2026-06-21
---

# Sound Instructions

The following will introduce the basic instructions used to play sound effects and BGM. There are also lots of other applied instructions. Please refer to the sample programs for information on how to use them.

## Play a Sound Effect - BEEP Instruction

The `BEEP` instruction we talked about on the page describing DIRECT mode is used to play short sound effects. Using this instruction, you can play a sound effect chosen from the list of various preset effects by specifying the effect number.

**Format**
```
BEEP Sound effect number
```

- Sound effect number should be in the range 0-133.
- Sound effect number can be omitted (If omitted = 0).

**Usage example**
```
BEEP 8
```

In applied usage, you can tune the frequency, volume, and pan-pot. Please input the instruction and check the description given in the HELP feature.

You can check the available sound effect numbers and their contents from the "BEEP" list, which is displayed when the SMILE button is pressed.

## Play BGM - BGMPLAY Instruction

The `BGMPLAY` instruction allows you to easily play BGM in your programs, for example in games. This product provides 43 ready-to-use preset BGM pieces.

**Format**
```
BGMPLAY Track number, BGM number
```

- Track number: Specifies the target track when multiple tunes are played at the same time (0-7)
- BGM number: Specifies the number of the preset tune to play (0-42) (Optional; 128-255 are user defined tunes.)

**Usage example**
```
BGMPLAY 12
```

Some preset tunes will end after being played once, while others will loop until the `BGMSTOP` instruction, described below, is executed.

You can check the available BGM numbers and their contents from the "BGM" list, which is displayed when the SMILE button is pressed.

## Stop BGM - BGMSTOP Instruction

This instruction is used to stop BGM that is currently playing.

**Format**
```
BGMSTOP
```

- Immediately stops the music on all tracks

If you specify the target track, you can also make the music fade out before stopping.

**Format**
```
BGMSTOP Track number, Fade-out time
```

- Track number: Track on which to stop the music (0-7)
- Fade-out time: Number of seconds for which to gradually decrease the volume before stopping the music

If 0 or no value is specified, the music will stop immediately.

**Usage example**
```
BGMSTOP 0,2
```

Input and run the following program:

The first line starts playing BGM No. 12 (As the track number is omitted, the BGM is played on track 0). The second line is an instruction called `WAIT`, which waits for a specified amount of time (in units of 1/60th of a second). "WAIT 60" waits for one second. The `WAIT` instruction used here waits for 5 seconds. The third line fades out the BGM playing on track 0 for 3 seconds before stopping it.

As a result, the BGM is played, then after 5 seconds it starts to fade out, and then after 3 seconds it stops completely.

## Play a musical scale - BGMPLAY Instruction

You can also play music using MML (Music Macro Language), which was used in old versions of BASIC.

Execute the following instruction:

This is a method for playing music where a character string enclosed in double quotations ("") is interpreted as a score. Each alpha character represents a note.

### BGMPLAY instruction (Play MML)

**Format**
```
BGMPLAY "MML"
```

- MML: Character string to play, which is a simplified representation of a score (See below)

**Usage example**
```
BGMPLAY ":0CCC :1REE :2RRG"
```

Uses three channels to play a chord

### Main MML Elements

- **A thru G** — Note (Play a sound)
  - C(Do)  D(Re)   E(Mi)  F(Fa)   G(Sol)  A(La)  B(Si)
  - Examples of musical scale notes

- **# or +** — Halftone up
  - C# D# E# F# G# A# G#
  - C+ D+ E+ F+ G+ A+ G+

- **- (minus)** — Halftone down
  - C- D- E- F- G- A- B-

- **R** — Rest

- **Number after a note** — Tone length
  - C1  C2  C4  C8  C16  C32
  - From left, Whole note, Half note, Quarter note, Eighth note, Sixteenth note, and Thirty-second note respectively

- **. (period) after tone length** — Dotted note
  - C1.  C2.  C4.  C8.  C16.  C32.
  - Increases each tone length by half

- **Lx** — Default tone length
  - (where x is a number of a tone length)

- **Ox** — Octave of a tone
  - (where x is a range from 0 thru 8)

- **<** — One octave up

- **>** — One octave down

- **:x** — Channel
  - (where x is a range from 0 thru 15)

- **Tx** — Tempo
  - (where x is a range from 1 thru 240)

- **Vx** — Change in volume
  - (where x is a range from 0 thru 127)

- **@x** — Change in tone
  - (where x is a range from 1 thru 127)

By inputting MML and characters and then pressing the HELP button, you can view detailed explanations for MML.
