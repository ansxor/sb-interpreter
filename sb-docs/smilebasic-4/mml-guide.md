---
title: MML Guide
slug: docs-sb4-mml-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-mml-guide
content_id: 19568
created: 2023-03-22
scraped: 2026-06-21
---

# MML Guide

*Music Macro Language (or MML)* is a markup language for writing music. You can think of it like an ASCII version of sheet music. SmileBASIC uses its own variant of MML for user-programmable background music. There is no MML standard, but it was originally introduced in classic BASIC systems and implementations of it are still used (e.g. in game engines.) This guide details the syntax and usage of SmileBASIC 4's MML engine.

## Overview

An MML composition consists of musical notes, rests, instrument selections, and other control sequences represented as normal text. MML code is written in strings and the [`BGMPLAY`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmplay), [`BGMSET`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmset), and [`BGMSETD`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmsetd) functions are used to read strings of MML and either store them in user song slots or play them immediately.

Basic knowledge of music is assumed for this guide. Keep in mind, though, that MML is not *just* ASCII sheet music. It is a language that describes music playback. Measures, time signatures, key signatures etc. are not defined by MML. Any arrangement of valid notes in any tempo will play.

### Examples & Comments

The examples are written in MML code directly, not surrounded in strings or [`DATA`](https://smilebasicsource.com/forum/thread/docs-sb4-data) statements, for convenience of reading. The code block syntax highlighting may make them look odd (it is currently designed /only/ for SmileBASIC code,) but the coloring is not important.

In MML, comments can be written by surrounding text in `/` symbols. This convention will also be used where necessary.

```mml
/this is an example/
CDEFG
```

### Playing MML

When exploring short examples, [`BGMPLAY`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmplay) is the easiest way to do it. The MML code is passed directly to the function as a string argument.

```sb4
BGMPLAY "CDEFGAB"
```

There are a variety of BASIC functions related to MML and BGM playback, but they're not the focus of this page. Check out the documentation pages in the Music section for more info.

## Notes & Rests

Notes are the basic building blocks of a song, so MML has lots of flexible ways of writing notes. A note can be written using the letters of the classic Western scale. This specifies the basic pitch of the note.

```mml
/the notes of the classic scale/
CDEFGAB
```

A rest (a silent note) is written with the letter `R`.

```mml
/a rest/
R
```

Sharps can be written by putting a `+` or `#` symbol after the note, and flats can be written by writing a `-` after.

```mml
/C-sharp/
C+
C#
```

```mml
/C-flat/
C-
```

> Remember: SmileBASIC is case-insensitive, so `Cb` is not a C-flat, it's C then B.

MML does not care about key signatures. You can use flats and sharps however you want (but it is up to you to use them properly if you have a certain key in mind!)

### Note Duration

By default, all notes are quarter notes (though the default note duration can be changed, which is explained later.) To specify a different duration per note, you can write a number after it.

```mml
/whole notes/
C1D1E1
/eighth notes/
C8D8E8
/a half rest/
R2
```

Only specific note durations are valid: 1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, and 192.

You can write dotted notes, by writing `.` after the note or its duration. A dot increases the note's duration by half (e.g. a dotted quarter is as long as a quarter plus an eighth.) Successive dots add progressively smaller duration increases. Dotted notes are more common in traditional sheet music.

```mml
/these notes get slightly longer/
C
C.
C..
```

If it's more convenient, the length for upcoming notes can be set using the `L` command. Remember this affects all upcoming notes, not just the next note.

```mml
/use eighth notes next/
L8CDE
```

### Octaves

An *octave* is the range of pitches covered by the standard eight-note scale. By default, the pitch letters are assigned to the middle octave, 4 (so `C` is "middle C.") Changing octaves allows you to use a much wider range of pitches. To specify the octave to use for upcoming notes, the `O` command is used. Octaves can be specified from -1 to 9.

```mml
/the standard scale again/
CDEFGAB
/the standard scale on the next octave up/
O5CDEFGAB
```

You can also step up an octave with `<` or down an octave with `>`. Remember, octave commands affect all upcoming notes, like `L`.

```mml
/finally, the full scale/
CDEFGAB<C
/and down this time/
C>BAGFEDC
```

An octave step command is translated to a normal octave command, relative to the current octave setting.

```mml
O4 C<C
/is the same as/
O4 C O5 C
```

This distinction is important when using these commands in loops (which will be explained later.)

### Slur & Tie

Writing a `&` between two notes connects them. If the two notes are the same pitch, it's a *tie*. These two notes are treated as one note played for the entire duration. If the two notes are different, it's a *slur*. The notes are played as though they're connected. Many notes can be tied together in order.

```mml
/tie/
C4&C8
```

```mml
/slur/
C&E&G
```

A tie can also be written by writing just a note length after the `&` symbol.

```mml
C4&8
/is the same as/
C4&C8
```

### Portamento

Writing a `_` between two notes creates a portamento (or "slide") effect. The pitch smoothly changes over the duration of the note from the first to the second.

```mml
/slide from C to D/
C_D
```

Portamento can behave strangely when used with note durations, so experiment with what sounds best.

```mml
C_D8
C8_D8
C4_D8
```

### Pitch Numbers

If you want to, you can specify pitch numbers directly with `N`. The range is from 0 to 127 and corresponds to pitch numbers used by MIDI. It is not usually convenient, but can be used to transcribe directly from MIDI.

```mml
/60 is middle C/
N60
/duration is written after a comma/
N60,8
```

### Chords

Chords can be written by surrounding a group of notes with `|`. All notes inside of the chord are played simultaneously.

```mml
/a CEG chord/
|CEG|
```

A chord is treated like a single note. It is affected by duration commands, dots etc. Octave commands can be used inside of a chord and will only take effect within it.

```mml
/the outer C is still octave 4/
|CO5C| C
```

Remember: a chord can contain *only* notes. No other MML commands can be used inside. You also can't include `N` commands, or connect two chords with slurs, ties, or portamentos.

## Channels

An MML composition can contain up to 16 *channels.* A channel is a separate sequence or stream of MML commands, with its own notes, properties etc. Channels are specified using `:`.

```mml
:0 CDE
:1 FGA
```

All defined channels play simultaneously. This allows you to write music with multiple instrumental parts, for example. Because channels are just separate MML streams, they are not restricted to single instruments (or even playing one note at a time, if you use chords.) All channels are totally independent (for the most part) which makes MML extremely versatile.

Keep in mind, most MML commands affect only the channel they're written in, and usually only for the notes following them.

If channel definitions are not used, then channel 0 is used by default. If you do use channels, you should not put anything before a channel statement, except for commands that affect the entire song, comments, or macro definitions. This can cause confusing MML.

```mml
/channel 0 is CCDE/
C
:0 CDE
:1 DEF
```

Channels can be defined only once, but can be defined in any order. As far as we know, the channel order does not affect sound priority or playback. An odd outlier to this rule is channel 0, due to its default usage. If it is defined after any other channel, it is considered an error, even if no content exists in channel 0.

```mml
/this MML will not load because channel 0 is "duplicate"/
:1 C
:0 D
```

It is best to just use channel numbers in order, starting from 0, as you need them, and only put global song commands before channel definitions. This keeps your MML well-structured and avoids errors.

## Instrument Selection

The `@` symbol followed by a number is used to select what instrument (or sound sample) is used for upcoming notes.

| ID Range | Description |
| --- | --- |
| @0-@127 | GM Sound Source Equivalent (can be confirmed with SMILETOOL) |
| @128 | Standard1 Drum Set |
| @129 | Electric1 Drum Set |
| @130 | PSG Drum Set |
| @131 | Standard2 Drum Set |
| @132 | Standard3 Drum Set |
| @133 | Room Drum Set |
| @134 | HipHop Drum Set |
| @135 | Jungle Drum Set |
| @136 | Techno Drum Set |
| @137 | House Drum Set |
| @138 | Power Drum Set |
| @139 | Electric2 drum set |
| @140 | BOB Drum Set |
| @141 | Dance Drum Set |
| @142 | QOQ Drum Set |
| @143 | Jazz Drum Set |
| @144 | Brush drum set |
| @145 | Orchestra Drum Set |
| @146 | Ethnic Drum Set |
| @147 | Asia Drum Set |
| @148 | Unn Drum Set |
| @216-@222 | PSG Sound Source |
| @223 | Noise |
| @224-@255 | User-Defined Waveform (waveform registered with WAVSET) |
| @256-@432 | Sound Effects Prepared for BEEP and etc. |

```mml
/use a Celesta sound/
@8C
```

### Drum Kits

Instrument numbers 128 through 148 instead refer to drum kits. When a drum kit is used, the different pitches map to different percussion samples instead of pitches of the same sound.

## Tempo

The `T` command changes the song's tempo. This command can be used within or outside channels, and affects all channels simultaneously (all channels play at the same tempo.) The value is specified in beats (quarter notes) per minute, in the range 1 to 1023.

```mml
T200 CDEFGAB<C
```

## Key Transpose

The `K` command adjusts the key of the song or channel, by transposing notes by a number of semitones. If used before a channel definition, the transpose applies to the entire song. If used within a channel, it applies to upcoming notes within that channel. The parameter is in the range of -115 to 115 semitones.

```mml
/make the song flat (decrease by one semitone)/
K-1
:0 CDE
:1 CDE
```

```mml
/make one channel sharp (increase by one semitone)/
:0 K1 CDE
:1 DEF
```

## Detune

The `@D` command applies a "detuning" effect, adjusting the pitch of upcoming notes. The parameter range is from -128 to 127. Negative values decrease pitch, and positive values increase it. 64 represents a pitch change of one semitone. This command is similar to `K`, but adjusts by 1/64th of a semitone.

```mml
@D12 CDE
```

## Volume and Velocity

In MML there are two related settings called *volume* and *velocity*. They both affect how loud an instrument is, but they are subtly different.

### Channel Volume

The `V` command changes the channel's volume. The range is 0 to 127, and 127 is the default.

```mml
/change the channel volume/
V96 C
```

Volume can also be increased or decreased with `(` and `)`.

```mml
CDE ( FGA ) B<C
```

Writing a number after the symbol will specify by how much the volume should be changed.

```mml
CDE (30 FG
```

These commands are essentially converted to normal `V` commands, relative to the current volume setting.

```mml
V64 C (32 D
/is the same as/
V64 C V96 D
```

This distinction is important to know when using loops (which will be mentioned later.)

### Channel Velocity

The `@V` command changes the velocity. Like volume, the range is 0 to 127.

```mml
/change the velocity/
@V96 C
```

### What is the difference?

Velocity represents the "force" or loudness a real instrument would be played with, like how hard you press the keys on a piano. Volume just represents how loud the channel's sound should be mixed. In MIDI, velocity settings can affect the sound of an instrument (such as using a different audio sample) while volume just changes how loud the channel is. In SB4 it's unclear if velocity actually corresponds to a difference in sound or if it's just a separate volume setting.

What you should know is how volume and velocity interact. Since volume is the volume of the entire channel, it affects how loud the loudest note velocity will be. So, `@V127` will still be very quiet if you use `V16`. In terms of loudness, velocity is essentially a percentage of the channel volume, where 0 is "silent" and 127 is "as loud as I can go at this volume." You should use velocity as a temporary setting for specific notes, and volume either once, or at specific portions in a song to change the volume.

### Song Volume

You might notice that there is no volume setting for all channels, like tempo or key. The song volume is set at playback with [`BGMPLAY`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmplay) or changed while playing with [`BGMVOL`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmplay).

## Gate Time/Staccato

The `Q` command changes the "gate time" of upcoming notes, or how much of the note's duration is actually audible. The value is in the range from 0 to 32, where 0 is the minimum (notes are cut very short) and 32 is the note's full duration. "Cut off" notes are traditionally called /staccato/.

```mml
/these quarter notes sound cut off/
Q8CDEF
```

## Pan

The `P` command affects the stereo pan of notes. 0 is furthest left, 64 is center, and 127 is furthest right. 64 is the default.

```mml
C P0 D P127 E
```

## Envelope Setting

The `@E` command modifies the *ADSR envelope*.

### What is an ADSR Envelope?

ADSR stands for Attack, Decay, Sustain, and Release. These four parameters affect how the volume of a sound can change over its playback duration.

- *Attack* is how quickly the sound reaches its /peak volume/ (a "fade in" setting).
- *Decay* is how quickly the sound falls from its peak volume to its /sustain volume/.
- *Sustain* is a volume setting (relative to volume/velocity) used in the middle of the envelope.
- *Release* is how quickly the sound goes from its sustain volume to silence (a "fade out" setting).

This is a concept used in all sorts of synthesizers, but this page will focus specifically on how it works in MML.

The description of each parameter is very important. Attack, decay, and release are /rates/, not units of time. (I don't know exactly what the values correspond to in MML logic.) 0 means "as slow as possible," and 127 means "instant." Importantly, these rates are not relative to the duration of the note. A low attack rate means the note may never reach its peak volume. A release rate of 0 means a note will *never end and cannot be stopped with [`BGMSTOP`](https://smilebasicsource.com/forum/thread/docs-sb4-bgmstop)*.

Similarly, sustain is a volume level relative to the current volume and velocity settings. Like volume its range is 0 to 127, with 0 being silent and 127 being the peak volume. The default setting for all parameters is 127, so instruments sound normal by default.

### Specifying the Envelope

`@E` changes the envelope by taking the four parameters, separated by commas.

```mml
/change the envelope setting/
/attack 110/
/decay 96/
/sustain 100/
/release 100/
@E110,96,100,100 C
```

It's also important to know that all instruments (even user-defined ones) have a pre-programmed ADSR envelope. The channel envelope setting is applied relative to an instrument's programmed envelope, so the sound of instruments is not significantly affected.

### Reset the Envelope

The `@ER` command resets the envelope setting to its default.

```mml
@E110,96,100,100 C
@ER D
```
