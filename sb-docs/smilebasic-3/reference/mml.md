---
title: MML Reference
slug: docs-sb3-mml
system: SmileBASIC 3
type: reference
category: MML
source: InstructionList.pdf
scraped: 2026-06-21
---

# MML Reference

MML (Music Macro Language) is the notation system used for composing and controlling music in SmileBASIC 3, primarily with `BGMSET` and music playback commands. It provides low-level control over tone, pitch, volume, instruments, effects, and timing.

## Commands for controlling whole tunes

| Command | Meaning |
|---------|---------|
| `:0` - `:15` | Channel specification (colon followed by channel number). Example: `BGMPLAY "T120:0CCC:1EEE:2GGG"` |
| `T1` - `T512` | Tempo specification (beats per minute/unit). Example: `T120` sets tempo to 120. |

## Commands and notations for controlling tone length

| Command | Meaning |
|---------|---------|
| `L1` - `L192` | Default tone length specification; affects all subsequent tones until changed. |
| `C1`, `C2`, `C4`, `C8`, `C16`, `C32` | Individual tone length specification (apply after pitch symbol). Examples: `C1` (whole note), `C4` (quarter note), `C8` (eighth note). |
| `C1.` - `C32.` | Dotted note representations (dotted versions of the above lengths). |
| `&` | Connects the preceding/succeeding notes. |
| `_` | Portamento. |
| `Q0` - `Q8` | Note duration ratio (gate) setting. Smaller numbers produce a greater impression of breaks between successive tones. |

**Note:** Triplets should be specified as `C12C12C12`, `C24C24C24`, etc.

## Commands for controlling (tone) pitch

| Command | Meaning |
|---------|---------|
| `C` | Do |
| `D` | Re |
| `E` | Mi |
| `F` | Fa |
| `G` | Sol |
| `A` | La |
| `B` | Ti |
| `C#`, `D#`, `E#`, `F#`, `G#`, `A#`, `B#` | Halftone higher. |
| `C-`, `D-`, `E-`, `F-`, `G-`, `A-`, `B-` | Halftone lower. |
| `R` | Rest (can be used in the same way as scales). Example: `R4` (quarter-note rest). |
| `O0` - `O8` | Octave number specification. Example: `O4C=60` (one increment/decrement per halftone). |
| `<` | One octave up. |
| `>` | One octave down. |
| `!` | Inversion of octave specification; causes `<>` symbols to be handled in reverse. |
| `N0` - `N127` | Key value specification (absolute pitch by MIDI note number; `O4C=60`). |

**Example:** `BGMPLAY "CCDEFGGABBBB"` (Do Do Re Mi Fa Sol Sol La Ti Ti Ti Ti)

## Commands for controlling sound volume and localization

| Command | Meaning |
|---------|---------|
| `V0` - `V127` | Volume level value specification. |
| `(` | One volume level up. |
| `)` | One volume level down. |
| `P0` - `P127` | Pan pot specification (localization). Left: `P0`–`P63`; Center: `P64`; Right: `P65`–`P127`. |
| `@E` + `A,D,S,R` values | Envelope setting (volume change from sound generation to attenuation). Example: `@E127,100,30,100` |
| `@ER` | Envelope resetting (releases the envelope). |

### Envelope parameters

- **A (Attack time):** 0–127 (smaller value = slower onset)
- **D (Decay time):** 0–127 (smaller value = slower decay)
- **S (Sustain level):** 0–127
- **R (Release time):** 0–127 (smaller value = slower release)

## Commands for controlling tone changes

| Command | Meaning |
|---------|---------|
| `@0` - `@127` | Instrument sound (equivalent to GM; can be checked using SMILETOOL). |
| `@128` | Standard drum set. |
| `@129` | Electric drum set. |
| `@144` - `@150` | PSG sound sources. |
| `@151` | Noise sound source. |
| `@224` - `@255` | User-defined waveforms (registered with `WAVSET`). |
| `@256` | Sound effects provided for `BEEP`. |

### @128 Standard drum set

| Note | Drum Sound |
|------|-----------|
| `B1` | Acoustic Bass Drum 2 (909BD) |
| `C2` | Acoustic Bass Drum 1 (808BDTom) |
| `C2#` | Side Stick (808RimShot) |
| `D2` | Acoustic Snare (808SD) |
| `D2#` | Hand Clap |
| `E2` | Electric Snare (909SD) |
| `F2` | Low Floor Tom (808TomLF) |
| `F2#` | Closed Hi-hat (808CHH) |
| `G2` | High Floor Tom (808TomF) |
| `G2#` | Pedal Hi-hat (808CHH) |
| `A2` | Low Tom (808TomL) |
| `A2#` | Open Hi-hat (808OHH) |
| `B2` | Low-Mid Tom (808TomLM) |
| `C3` | High Mid Tom (808TomHM) |
| `C3#` | Crash Cymbal 1 (808Cymbal) |
| `D3` | High Tom (808TomH) |
| `D3#` | Ride Cymbal 1 |
| `E3` | Chinese Cymbal |
| `F3` | Ride Bell |
| `F3#` | Tambourine |
| `G3` | Splash Cymbal |
| `G3#` | Cowbell (808Cowbell) |
| `A3` | Crash Cymbal 2 |
| `A3#` | Vibra-slap |
| `B3` | Ride Cymbal 2 |
| `C4` | High Bongo |
| `C4#` | Low Bongo |
| `D4` | Mute Hi Conga (808CongaMute) |
| `D4#` | Open Hi Conga (808CongaHi) |
| `E4` | Low Conga (808CongaLo) |
| `F4` | High Timbale |
| `F4#` | Low Timbale |
| `G4` | High Agogo |
| `G4#` | Low Agogo |
| `A4` | Cabasa |
| `A4#` | Maracas (808Maracas) |
| `B4` | Short Whistle |
| `C5` | Long Whistle |
| `C5#` | Short Guiro |
| `D5` | Long Guiro |
| `D5#` | Claves (808Claves) |
| `E5` | Hi Wood Block |
| `F5` | Low Wood Block |
| `F5#` | Mute Cuica |
| `G5` | Open Cuica |
| `G5#` | Mute Triangle |
| `A5` | Open Triangle |

## Special effect commands

| Command | Meaning |
|---------|---------|
| `@MON` | Start modulation. |
| `@MOF` | Stop modulation. |
| `@D-128` to `@D127` | Detuning (fine frequency adjustment). `-128` is a tone lower; `+127` is a tone higher. |
| `@MA` + `Depth,Range,Speed,Delay` | Tremolo setting (all values 0–127). Example: `@MA64,1,16,32` |
| `@MP` + `Depth,Range,Speed,Delay` | Vibrato setting (all values 0–127). Example: `@MP64,1,16,32` |
| `@ML` + `Depth,Range,Speed,Delay` | Auto pan pot setting (all values 0–127). Example: `@ML100,1,8,0` |

**Note:** `@MA`, `@MP`, and `@ML` cannot be used at the same time.

## Special music playback commands

| Command | Meaning |
|---------|---------|
| `[` | Repeat start. |
| `]` or `]N` | Repeat end (N = number of times; if omitted, loop is endless). Example: `[[CCC]2DEF]2` plays `CCC CCC DEF CCC CCC DEF`. |
| `$0` - `$7` | MML internal variable reference. Can be used in place of numerical values in most commands (marked with ◆). Example: `$0=64 V$0` instead of `V64`. |
| `$0=value` - `$7=value` | Assign values (0–255) to MML internal variables. Variables can be assigned or referenced with the `BGMVAR` instruction during playback. |
| `{Label name=MML}` | Macro definition (up to eight alphanumerical characters). Channel specification within the defined MML is not allowed; reusing a label name is not allowed. |
| `{Label name}` | Macro use (expands MML corresponding to the defined label). |

**Example:** `BGMPLAY "T240@128O2{PT0=CDEDCDE<G}[{PT0}]4"` (play a rhythm using a macro).
