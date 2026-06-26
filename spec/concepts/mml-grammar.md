---
title: MML grammar
slug: mml-grammar
area: audio
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/mml.md (full SB3 MML command list — channels, tempo, lengths/gate, pitch/octave/key, volume/pan/envelope, instruments, modulation/detune/tremolo/vibrato/autopan, repeats, $-vars, {macros}, @128 drum map)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/sound-instructions.md (BGMPLAY MML intro; simplified ranges T1-240/@1-127/O0-8)" }
  - { type: documented, ref: "sb-docs/smilebasic-4/mml-guide.md (cross-system: @V velocity 0-127; tie '&len' short form; valid length divisors; portamento; chords '|..|' — SB4-only, flagged below)" }
  - { type: disassembled, ref: "cia_3.6.0.lst BGMPLAY handler @0x1a2d54: argcount at [r0,#0xc]; 0 args -> mov r0,#0x4 (errnum 4) @0x1a2d74; (argcount-1) >= 3 -> mov r0,#0x4 @0x1a2d9c (so 1..3 args legal); string arg path bl 0x1d44d8 (MML validate -> 0x1d475c) @0x1a2e20, on failure bl 0x1d450c (build '^' caret context, 0x5e='^') then mov r0,#0x2f -> errnum 47 Illegal MML @0x1a2e3c; number arg path @0x1a2e5c: sub r1,r0,#0x80 / cmp #0x7f -> 128..255 = user BGM, else cmp #0x2a (42) -> 0..42 = preset BGM, >42 -> error", confidence: disassembled }
  - { type: community, ref: "sbsave corpus: @V velocity in 196 programs (e.g. 1DVENVAE/TXT/PICS_R1, 2D7X32KV/TXT/BGM); SFX instruments @256..@287+ (e.g. @267 1624x, @275, @281, @287) beyond the documented single @256, and even higher @400-@411 attested; PSG @144-@149 + @D detune + @E envelope + Q gate + {macro}/$var forms all attested" }
  - { type: community, ref: "sbsave corpus (M5-T1 parser sweep, 541/550 complete BGM* literals parse): (a) VOLUME STEP WITH OPERAND `(N`/`)N` — docs show only operand-less `(`/`)`, but `)80`/`)24`/`(3`/`)5` pervasive (20+ programs, e.g. 4KHEPXW3/TXT/3DPARKOUR `C&)16C&(16`, BGMSET 222 `{CD=)24D&(3D…}`); read as change-volume-by-N-steps, bare = 1. (b) CASE-SENSITIVE MACRO LABELS — programs define BOTH `{r=…}` and `{R=…}` as distinct macros (e.g. 4KHEPXW3 has {r},{R},{c},{C0},{b},{B0},{a},{A0}), so MML is NOT globally upper-cased the way the docs imply; notes/commands stay case-insensitive. (c) DOTTED DEFAULT LENGTH `L<n>.` — `L2.`/`L8.` set a dotted default (7+ programs, e.g. L2.C ×117). (d) LEADING ACCIDENTALS `+B`/`#F`/`-C` before a note — 62+ programs (e.g. EK3E9F/TXT/PARKOUR BGMSET 145 `A8.B8.+B8`). All output-unproven → oracle-pending (bd:sb-interpreter-i8p M5-T1)." }
confidence: disassembled
related:
  - BGMPLAY
  - BGMSET
  - BGMSETD
  - BGMVAR
  - BGMSTOP
  - BGMCHK
  - BGMPRG
  - WAVSET
  - WAVSETA
---

# MML grammar

The contract for M5's MML front-end (`M5-T1` parser, feeding the `M5-T2` synth). **MML**
(Music Macro Language) is the text notation SmileBASIC uses to describe music. It is read by
the BGM commands — `BGMPLAY "<mml>"` plays it immediately; `BGMSET no,"<mml>"` /
`BGMSETD no,"<mml>"` compile it into a user song slot (128–255) for later
`BGMPLAY track,no`. This file specifies the **lexical/grammar layer only** (string → per-channel
event stream); the synth's sample-rate, mixing and envelope *curves* are M5-T2's concern and
are **not** in this model.

> **Deterministic gate.** Audio has no emulator golden (real-time, timing-dependent — see
> `prd/oracle.md` O-T7). The verifiable contract for M5 is therefore **MML → note-events**:
> given an MML string, the parser must emit a deterministic per-channel sequence of
> (command | note(pitch, ticks, …)) events. That mapping — everything in this file — is
> checkable against docs + disassembly without ever rendering audio.

## Where MML is parsed, and how it fails

`BGMPLAY` (handler `@0x1a2d54`) takes **1–3 arguments**; 0 args or >3 args raise
**errnum 4 (Illegal function call)** (`mov r0,#0x4` at `@0x1a2d74` / `@0x1a2d9c`). The first
argument is overloaded:

- **string** → treated as MML. It is validated by the MML parser (`bl 0x1d44d8` →
  `0x1d475c`). On any parse error the handler builds an error-context string with a `^` caret
  (`0x5e`) marking the offending position (`bl 0x1d450c`) and raises **errnum 47
  (Illegal MML)** (`mov r0,#0x2f` at `@0x1a2e3c`). MML strings are **single-arg only** for the
  string form.
- **number** → a BGM song id, not MML. The range guard at `@0x1a2e5c` accepts **0–42** (preset
  songs; `cmp #0x2a`) and **128–255** (user songs registered by `BGMSET`/`BGMSETD`;
  `sub #0x80`/`cmp #0x7f`). Anything 43–127 or >255 errors. The two- and three-arg forms are
  `BGMPLAY track(0–7), no` and `BGMPLAY track, no, volume`.

So **the grammar below is what must parse without raising errnum 47.** Any construct the real
parser rejects is illegal MML; any byte sequence it accepts is legal even if the docs omit it
(corpus forms below are proof of legality, never of rendered output).

## Lexical rules

- **Case-insensitive *for notes and commands*.** `cdefg` ≡ `CDEFG`. (Note the SB4
  caveat: `Cb` is `C` then `B`, *not* C-flat — flats are written `C-`.) **Macro labels are
  case-SENSITIVE**, however: the corpus has programs defining both `{r=…}` and `{R=…}` as
  distinct macros, so MML is not globally upper-cased the way "SmileBASIC upper-cases the
  source" implies — only command/note dispatch folds case (community, oracle-pending).
- **Whitespace** (spaces) between commands is ignored and may be used freely for readability
  (`":0 @7 V80 O4 G16C16E8"` is attested in the corpus).
- A command is a letter (or `@`/symbol) followed, where applicable, by its numeric operand
  read greedily as decimal digits. `C16` is one C note of length 16; `O4C` is octave-4 then C.
- **No string interpolation in MML** — values come from literal digits or `$n` MML variables
  (below), not from BASIC expressions. The MML string is built by ordinary BASIC string
  concatenation before it reaches `BGMPLAY` (the corpus is full of `…V"+STR$(v)+…` forms).

## Channels

| Token | Meaning |
|-------|---------|
| `:0` – `:15` | Select the channel that subsequent commands write to. Up to **16** independent channels. |

All channels play simultaneously, each its own stream with its own state (instrument, octave,
volume, …). If no `:` appears, everything is channel 0. Per the SB4 model (cross-system, to be
oracle-confirmed for SB3): channel 0 is implicit, so defining `:0` *after* another channel is
an error; channels may otherwise be defined in any order, once each. **Global** commands
(tempo, macros, comments) should precede the first channel statement.

## Tempo, note length, gate, ties

| Token | Range | Meaning |
|-------|-------|---------|
| `T`*n* | **1–512** (reference) | Tempo in beats (quarter notes) per minute. *(The e-manual's simplified intro says 1–240; the reference table and corpus `T512`/`T500`/`T501` confirm the true ceiling is 512.)* |
| `L`*n*`[.]` | 1–192 | Default note length = a 1/*n* note; applies to all later notes until changed. May be **dotted** (`L2.`, `L8.` — corpus, 7+ programs; community, oracle-pending) — a length-less note then inherits the dotted default. |
| *(len after a note)* `C8` | 1–192 | Per-note length override (`C1`=whole, `C4`=quarter, `C8`=eighth, …). |
| `.` (after note or length) | — | Dotted note: +½ the base duration. Successive dots add progressively smaller halves (`C..`). |
| `&` | — | Tie / slur: connect this note to the next. `C4&C8` = one note held for 4+8. Same pitch = tie; different pitch = slur. Short form `C4&8` ≡ `C4&C8` (operand-less second note inherits pitch). |
| `_` | — | Portamento (pitch slides from the previous note to this one over its duration). |
| `Q`*n* | 0–8 | Gate ratio: fraction of the note length actually sounded before the next. Lower = more staccato (bigger gaps); `Q8` ≈ fully legato. |

**Length values & ticks.** Valid length divisors are those that divide a whole note evenly:
the well-known set is 1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 192 (cross-system from
SB4; SB3 reference shows `L1–L192` and the example set `C1/2/4/8/16/32` plus triplets
`C12/C24`). Triplets are written as same-length runs: `C12C12C12`. The natural internal
resolution implied by `L1–L192` is **192 ticks per whole note** (48 per quarter) — the LCM that
makes every listed divisor and its dotted form land on an integer tick. *(Tick base + the exact
T→frames conversion are **hypothesis**, pending a read of the synth scheduler — queued.)*

## Pitch, octave, key

| Token | Range | Meaning |
|-------|-------|---------|
| `C D E F G A B` | — | Notes Do Re Mi Fa Sol La Ti. |
| `#` or `+` (after note) | — | Sharp (semitone up). `C#` ≡ `C+`. **Also legal *before* the note** (`+C` ≡ `C+`, `#F`) — undocumented but in 62+ corpus programs (community, oracle-pending). |
| `-` (after note) | — | Flat (semitone down). `C-`. Also legal *before* the note (`-C`). |
| `R` | optional len | Rest (silence). Takes a length like a note: `R4`. |
| `O`*n* | 0–8 | Set absolute octave. Default **O4**, where `O4C` = key 60 (middle C). |
| `<` | — | Octave up by one. |
| `>` | — | Octave down by one. |
| `!` | — | Invert octave shifts: after `!`, `<`/`>` swap meaning. |
| `N`*n* | 0–127 | Absolute pitch by MIDI key number (`N60` = middle C); independent of `O`. |

Sharps/flats may freely exceed key boundaries (MML imposes no key signature). `<`/`>` are
relative to the current `O` setting, which matters inside repeats/macros.

## Volume, pan, envelope

| Token | Range | Meaning |
|-------|-------|---------|
| `V`*n* | 0–127 | Channel volume. |
| `(`*[n]* | — | Volume **up** by *n* steps (bare = 1). The docs show only the operand-less form, but `(N`/`)N` (`(3`, `)24`, `)80`) is pervasive in the corpus (20+ programs); read as change-by-N-steps. Step *size* and N's ceiling are oracle-pending (community). |
| `)`*[n]* | — | Volume **down** by *n* steps (bare = 1). |
| `P`*n* | 0–127 | Pan pot. **0–63** = left, **64** = center, **65–127** = right. |
| `@E`*A*,*D*,*S*,*R* | each 0–127 | ADSR envelope. A = attack time, D = decay time, S = sustain level, R = release time (for A/D/R, *smaller value = slower*). Example `@E127,100,30,100`. |
| `@ER` | — | Reset/release the envelope to default. |
| `@V`*n* | 0–127 | **Note velocity** — per-note loudness as a fraction of channel `V`. Undocumented in the SB3 reference but documented in SB4 (`@V96 C`) and pervasive in the corpus (196 programs). Distinct from `V`: `V` is the channel ceiling, `@V` scales individual notes under it. *(SB3 exact behavior oracle-pending — queued.)* |

## Instruments (`@`)

| Token | Meaning |
|-------|---------|
| `@0` – `@127` | GM-equivalent melodic instruments. *(The e-manual intro says `@1–127`; the reference and corpus `@0`/`@7`/`@120…@127` confirm `@0` is valid.)* |
| `@128` | Standard drum set (note→drum map below). |
| `@129` | Electric drum set. |
| `@144` – `@150` | PSG tone sources. |
| `@151` | Noise source. |
| `@224` – `@255` | User-defined waveforms (registered with `WAVSET`/`WAVSETA`). |
| `@256` | Sound-effect bank (the effects `BEEP` uses). |

> **Corpus note (community, oracle-pending):** programs use SFX instrument numbers well above
> the single documented `@256` — `@256`–`@287`+ are attested (e.g. `@267` in 1624 lines,
> `@275`, `@281`, `@287`). These appear to be additional SFX/voice bank entries. The documented
> list also omits the SB4-era extra drum kits (`@130`–`@134`); do **not** assume them for SB3
> until confirmed. Both are queued for oracle confirmation.

### `@128` standard drum map (note → drum)

| Note | Drum | Note | Drum | Note | Drum |
|------|------|------|------|------|------|
| `B1` | Acoustic Bass Drum 2 | `C3#` | Crash Cymbal 1 | `D5` | Long Guiro |
| `C2` | Acoustic Bass Drum 1 | `D3` | High Tom | `D5#` | Claves |
| `C2#` | Side Stick | `D3#` | Ride Cymbal 1 | `E5` | Hi Wood Block |
| `D2` | Acoustic Snare | `E3` | Chinese Cymbal | `F5` | Low Wood Block |
| `D2#` | Hand Clap | `F3` | Ride Bell | `F5#` | Mute Cuica |
| `E2` | Electric Snare | `F3#` | Tambourine | `G5` | Open Cuica |
| `F2` | Low Floor Tom | `G3` | Splash Cymbal | `G5#` | Mute Triangle |
| `F2#` | Closed Hi-hat | `G3#` | Cowbell | `A5` | Open Triangle |
| `G2` | High Floor Tom | `A3` | Crash Cymbal 2 | | |
| `G2#` | Pedal Hi-hat | `A3#` | Vibra-slap | | |
| `A2` | Low Tom | `B3` | Ride Cymbal 2 | | |
| `A2#` | Open Hi-hat | `C4` | High Bongo | | |
| `B2` | Low-Mid Tom | `C4#` | Low Bongo | | |
| `C3` | High Mid Tom | `D4` | Mute Hi Conga | | |
| | | `D4#` | Open Hi Conga | | |
| | | `E4` | Low Conga | | |
| | | `F4` | High Timbale | | |
| | | `F4#` | Low Timbale | | |
| | | `G4` | High Agogo | | |
| | | `G4#` | Low Agogo | | |
| | | `A4` | Cabasa | | |
| | | `A4#` | Maracas | | |
| | | `B4` | Short Whistle | | |
| | | `C5` | Long Whistle | | |
| | | `C5#` | Short Guiro | | |

(Full map in `sb-docs/smilebasic-3/reference/mml.md`.)

## Modulation / detune / tremolo / vibrato / autopan

| Token | Operands | Meaning |
|-------|----------|---------|
| `@MON` | — | Start modulation. |
| `@MOF` | — | Stop modulation. |
| `@D`*n* | −128 … 127 | Detune (fine pitch). `−128` ≈ one tone down, `+127` ≈ one tone up. |
| `@MA`*depth*,*range*,*speed*,*delay* | each 0–127 | Tremolo (amplitude LFO). |
| `@MP`*depth*,*range*,*speed*,*delay* | each 0–127 | Vibrato (pitch LFO). |
| `@ML`*depth*,*range*,*speed*,*delay* | each 0–127 | Auto pan (pan LFO). |

`@MA`, `@MP`, `@ML` are **mutually exclusive** — only one may be active on a channel at a time
(per the reference note). `@D`, `@E`, `@MA/@MP/@ML`, `@V` are all attested together in the
corpus (`@D-5@E113,102,0,118`, `@D5`, `@V70Q5`).

## Playback control: repeats, variables, macros

| Token | Meaning |
|-------|---------|
| `[` | Repeat start. |
| `]` or `]`*N* | Repeat end. `]N` repeats *N* times; bare `]` loops forever. Nestable: `[[CCC]2DEF]2` → `CCC CCC DEF CCC CCC DEF`. |
| `$0` – `$7` | Reference an MML variable in place of a numeric operand (the commands that accept this are marked ◆ in the reference). `$0=64 V$0` ≡ `V64`. |
| `$n=`*value* | Assign an MML variable (value **0–255**). Also settable at runtime with `BGMVAR`. |
| `{`*Label*`=`*MML*`}` | Macro **definition**. Label = up to 8 alphanumerics. The MML inside may **not** contain a channel (`:`) command, and a label may not be redefined. |
| `{`*Label*`}` | Macro **use** — expands the label's MML inline. |

Macros and `$`-vars are heavily used in the corpus (`{OTO=…}`, `{AM=…}`, `{R=…}`, `$0=224`).
`$`-vars bridge to BASIC via `BGMVAR no, var, value`, which can change a variable *during*
playback.

## Comments and chords — SB4-only, not assumed for SB3

The SB4 MML guide documents `/comment/` delimiters and `|note-group|` chords. **Neither appears
in the SB3 reference.** Treat them as **out of scope for SB3** (likely errnum 47) until the
oracle says otherwise — do not parse them in the SB3 engine on the strength of SB4 docs alone.
Queued for confirmation.

## Note-event model (what the parser emits)

For each channel the parser produces an ordered event stream. The deterministic conformance
tests (M5-T1) assert this stream, not audio. Each **note/rest** event carries:

- `pitch`: resolved MIDI key (from letter+accidental+octave, from `N`, or "rest"/drum-slot),
  after `<`/`>`/`!` and `@D` detune are folded in;
- `duration`: in ticks (192/whole-note base), after length, dots, and `&` ties are resolved;
- `gate`: sounded fraction from `Q`;
- `velocity`/`volume`/`pan`: current `@V`/`V`/`P`;
- `instrument` + any active envelope/LFO state.

Control commands (tempo, instrument change, repeat boundaries unrolled, macro expansion,
variable substitution) are resolved at parse time so the synth consumes a flat per-channel
timeline. Malformed input anywhere → **errnum 47**, with the byte offset of the failure (the
`^` caret position) available for diagnostics.

## Open questions (tracked in beads — bd:sb-interpreter-i8p)

- Exact tick base (192/whole-note assumed) and the precise T(tempo)→frames(60 fps) conversion —
  read the synth scheduler in the disassembly.
- `@V` velocity: confirm SB3 range (0–127) and how it scales against `V` (multiplicative %?).
- SFX instrument bank: confirmed upper bound above `@256` (corpus shows ≥`@287`); the `@130`–
  `@134` extra drum kits — present in SB3 or SB4-only?
- `!` octave-invert, `(`/`)` volume step size, and `Q` gate's exact tick formula.
- Channel-0-redefinition / channel-order rules — confirmed for SB3 (currently cross-system from SB4).
- `/comments/` and `|chords|` — does SB3 accept them (it appears not), or errnum 47?
- `$n` value range at assignment (docs say 0–255) and clamping behavior on overflow.
- **(M5-T1 corpus, oracle-pending)** the four community forms above: `(N`/`)N` step semantics
  (by-N-units? saturating at 0/127?) + N's ceiling; confirm macro labels are case-sensitive;
  dotted `L<n>.` semantics when a note adds its own dots; leading accidentals on a note. Also:
  is an accidental before a **rest** (`+R`, `#R`, seen in 1–2 programs) legal-and-ignored or
  errnum 47? The parser currently treats it as errnum 47.
