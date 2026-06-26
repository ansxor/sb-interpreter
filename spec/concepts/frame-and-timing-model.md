---
title: Frame & timing model
slug: frame-and-timing-model
area: input
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/vsync.md (VSYNC counts from the last VSYNC; 0=ignore; omitted=1)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/wait.md (WAIT counts from the present point; 0=ignore; omitted=1)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/system-variables.md (MAINCNT = frames since SmileBASIC launched; TIME$ HH:MM:SS; DATE$ YYYY/MM/DD)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/brepeat.md + fade.md (timing in units of 1/60th of a second)" }
  - { type: disassembled, ref: "cia_3.6.0.lst: one global frame counter at ptr [0x315ec0]. MAINCNT getter @0x15e5f8 does ldr r0,[0x15e604](->0x315ec0)/ldr r0,[r0,#0] -> returns *[0x315ec0]. WAIT handler @0x14afb0 targets current+count from [0x315ec0]; VSYNC handler @0x1455c8 targets lastVsync+count from [0x315ee8] then writes current back to [0x315ee8]. Both yield per frame with swi 0xa and check count<=0 to skip. The three xrefs of 0x315ec0 are exactly these handlers + the MAINCNT getter." }
  - { type: hw_verified, ref: "spec/instructions/VSYNC.yaml + WAIT.yaml: A=VSYNC(1)/A=WAIT(1) -> errnum 4 (sb-oracle batch 2026-06-22)" }
  - { type: hw_verified, ref: "MAINCNT is read-only (sysvars.yaml writable=false): assigning `MAINCNT=5` -> errnum 3 (Syntax error), errline 1 (sb-oracle batch 2026-06-23). M4-T3 rejects it at compile time, matching the ERRNUM/ERRLINE/ERRPRG read-only handling." }
confidence: disassembled
related:
  - VSYNC
  - WAIT
  - MAINCNT
  - TIME$
  - DATE$
  - BREPEAT
  - BUTTON
  - FADE
  - SPANIM
  - BGANIM
---

# Frame & timing model

The contract for M4's frame clock (and the shared time base that M3 animation and M5 audio
scheduling line up against). SmileBASIC runs at a fixed **60 fps**; almost every duration in
the language — VSYNC/WAIT counts, BREPEAT start/interval, FADE time, animation keyframe
times — is measured in **frames = 1/60th of a second** (documented across `vsync.md`,
`wait.md`, `brepeat.md`, `fade.md`). There is no sub-frame timer instruction (no MILLISEC);
wall-clock time is exposed only as the RTC strings `TIME$`/`DATE$`, which are a **separate**
clock from the frame counter (see below).

## The single global frame counter

There is exactly **one** free-running frame counter, a 32-bit value at pointer **`[0x315ec0]`**
(value pointer; the handlers load the pointer then dereference). It increments once per
displayed frame (60 Hz). Everything frame-timed reads or compares against it:

| Consumer | Disassembly | Role |
|---|---|---|
| **MAINCNT** sysvar | getter `@0x15e5f8` → `*[0x315ec0]` | returns the raw counter value |
| **WAIT** | handler `@0x14afb0`, ref `@0x14b084` | target = **current** counter + count |
| **VSYNC** | handler `@0x1455c8`, ref `@0x1456a0` | frame source for the wait loop |

The disassembly confirms the unification directly: `disasm.py xref 0x315ec0` returns *exactly
three* sites — the WAIT handler, the VSYNC handler, and the MAINCNT getter — so MAINCNT and
the VSYNC/WAIT pacing are reading the same number, not parallel clocks.

- **MAINCNT** = "frames since SmileBASIC was **launched**" (docs). It is **not** reset by
  `RUN`, `NEW`, `CLEAR`, or program start — it counts from boot and never stops. Type Integer
  (i32); it will wrap at `0x7FFFFFFF` (≈414 days of uptime — not reachable in practice, but the
  i32 wrap is the model). Read-only (`spec/reference/sysvars.yaml`: MAINCNT writable=false).
- Because MAINCNT is the same counter VSYNC/WAIT advance against, you can measure elapsed
  frames as `MAINCNT - start` and it stays consistent with how long a `VSYNC`/`WAIT` blocked.

## Per-program "last VSYNC" anchor

Separate from the global counter, there is a per-program **last-VSYNC frame stamp** at pointer
**`[0x315ee8]`**. This is the only state that distinguishes VSYNC from WAIT:

```
global frame counter   *[0x315ec0]   advances 60×/sec, shared, == MAINCNT
last-VSYNC stamp        *[0x315ee8]   per program, updated by VSYNC and WAIT on exit
```

- **VSYNC [n]** waits until `*[0x315ec0] >= lastVsync + n`, then stores the current counter
  into `lastVsync`. The target is anchored at the **previous** VSYNC, so a `VSYNC 1` loop holds
  a steady 60 fps even when the loop body's duration jitters — the jitter is absorbed into the
  same frame budget. (`@0x14563c` computes `add r5, lastVsync, count`; `@0x145690` writes
  current → `[0x315ee8]`.)
- **WAIT [n]** waits until `*[0x315ec0] >= current + n` — the target is anchored at the
  **moment WAIT runs**, so body jitter accumulates into the effective rate. (`@0x14b020`
  computes `add r5, current, count` using `[0x315ec0]`, *not* `[0x315ee8]` — that one
  register choice is the entire VSYNC/WAIT difference.) WAIT **also** updates `lastVsync` to
  the current counter on exit (`@0x14b078`), so a `VSYNC m` placed after a `WAIT n` measures
  its window from the end of the WAIT, not from some older VSYNC.

See [[VSYNC]] and [[WAIT]] for the full per-instruction contracts (argument defaults,
`count <= 0` short-circuit, the "used as a function → errnum 4" guard).

### Argument edge behavior (both, disassembled)

| Case | Behavior | Source |
|---|---|---|
| omitted argument | defaults to **1** frame (`mov r1,#0x1`) | `@0x1455ec` / `@0x14afd4` |
| `n = 0` | **does not wait**, but still resyncs `lastVsync` to current ("0: Ignore") | `count<=0` branch `ble` |
| `n < 0` | treated like 0 — `cmp`/`ble` skips the wait, no error raised | same branch |
| used as a function (`A=VSYNC(1)`) | **errnum 4** Illegal function call (return-count guard) | `mov r0,#0x4` `@0x1455e0` / `@0x14afc8`; hw_verified |

## The frame yield (`swi 0xa`)

While blocked, both VSYNC and WAIT yield a frame at a time with the 3DS **`swi 0xa`**
(wait-for-VBlank / GSP-event syscall) inside the wait loop (`@0x14567c` / `@0x14b060`). The
implications the model must preserve:

- The program stays **interruptible** during a wait — the BREAK/STOP check still runs each
  frame, so a long `WAIT 600` can be broken out of.
- Background machinery that ticks per frame keeps running across the wait: **MAINCNT keeps
  advancing**, sprite/BG animations advance, BGM plays, input state refreshes.
- A frame is the smallest schedulable unit. There is no busy-wait shorter than one frame and
  no way to wait a fractional frame.

In the headless/deterministic runner there is no real VBlank: the model is a single frame
clock that advances explicitly. `swi 0xa` becomes "advance the clock one frame and run the
per-frame tick (animation step, audio scheduler, input poll, BREAK check)"; VSYNC/WAIT spin
that tick until their target counter value is reached.

## How everything else hangs off the frame clock

The frame is the shared heartbeat for the other subsystems — all of these durations are
**frame counts**, so they advance in lockstep with MAINCNT and with VSYNC/WAIT:

- **Animation** ([[SPANIM]] / [[BGANIM]], see [[sprite-bg-model]]): keyframe times are in
  frames (positive = hold N frames, negative = interpolate over |N| frames); animation
  **starts on the frame *after*** the call. One animation step happens per frame tick.
- **Input repeat** ([[BREPEAT]]): start-time and interval are "in units of 1/60th of a second"
  (docs) — i.e. frames; [[BUTTON]]'s repeat mode counts the same frames.
- **[[FADE]]**: fading time is in 1/60ths of a second; the fade progresses one step per frame.
- **Audio scheduling** (M5): MML tempo/note durations resolve to sample counts, but BGM event
  dispatch and `BGMSTOP`/effect changes are quantized to the frame tick that drives everything
  else (the audio *render* is real-time and not part of this deterministic clock — see O-T7).

## Wall-clock vs frame clock (don't confuse them)

`TIME$` (HH:MM:SS) and `DATE$` (YYYY/MM/DD) read the **3DS real-time clock**, a clock that is
independent of the frame counter and of MAINCNT. They keep correct wall time even if frames
are dropped, and they are not affected by VSYNC/WAIT. They are read-only Strings
(`spec/reference/sysvars.yaml`: the only two `type: s` sysvars). The frame counter measures
*elapsed frames since launch*; the RTC measures *calendar time* — a program that needs a
monotonic frame timer uses `MAINCNT`, and one that needs the clock uses `TIME$`/`DATE$`.

## Corpus notes (real-program usage)

- Bare `VSYNC` (no argument, = `VSYNC 1`) is the overwhelmingly common form (~1834 occurrences,
  e.g. `1DE453HV/TXT/SPACE_MOLE`) — the canonical per-frame pacing idiom. Expression arguments
  occur (`VSYNC SPEED`, `VSYNC (I MOD 40)==39`).
- `WAIT` appears both bare and with computed counts: `WAIT D*60` (one second × D),
  `WAIT (RND(3)+1)*60`, `WAIT RND(20)`, `WAIT 0` — confirming the frame unit and the
  `0 = no-wait` case are used in real code. No `X=VSYNC(...)`/`X=WAIT(...)` return-value form
  appears in the corpus (consistent with the errnum-4 guard). *(community confidence — syntax
  proven by the corpus, semantics from docs/disasm.)*

## Open questions → oracle (tracked in beads — bd:sb-interpreter-7td)

- **Boot value / monotonicity of MAINCNT** across `RUN`/`NEW`/`CLEAR` and across a program
  halt+CONT — confirm it never resets and never pauses (docs say "since launched"; the i32
  wrap point is inferred, not observed).
- **VSYNC after a long body**: when the body already overran the target (e.g. body took 3
  frames before `VSYNC 1`), does VSYNC return immediately and advance `lastVsync` by the full
  overrun, or clamp to "current"? The `add lastVsync,count` vs the eventual `str current`
  suggests *catch-up* (lastVsync jumps to current on exit, dropping the missed frames) — pin
  the exact resync semantics on the oracle.
- **MAINCNT vs displayed-frame alignment** under DISPLAY/XSCREEN changes and during FADE —
  whether MAINCNT counts every VBlank regardless of what is shown (assumed yes).
- Sub-frame timing / any hidden high-resolution counter — none found in the disassembly;
  confirm there is genuinely no finer clock than the frame.
