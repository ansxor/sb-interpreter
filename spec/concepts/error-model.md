---
title: Error model
slug: error-model
area: execution
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/error-table.md (errnum 3..47; 'If an error has occurred, the relevant information is stored in the system variables ERRNUM (error number) and ERRLINE (the line where the error occurred).')" }
  - { type: documented, ref: "sb-docs/smilebasic-3/stop.md (STOP suspends; 'program SLOT:line number of the suspended program will be displayed'; resumable with CONT) + cont.md (CONT resumes from the suspend point set by START/STOP/error; cannot resume if edited, if waiting for input, or depending on the error type) + end.md (END exits the program / a DEF) + break.md (BREAK ends a loop)" }
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/system-variables.md (ERRNUM error number; ERRLINE line where the error occurred; ERRPRG program SLOT where the error occurred)" }
  - { type: disassembled, ref: "cia_3.6.0.lst: errnum->string ptr table @0x3054f8 (.data, 56 word-ptrs, errnum 0..55) -> ASCII pool @0x2e965c..0x2e9ac0 (.rodata). Error-message formatter FUN_001e94a8 @0x1e94a8: `sub r0,r0,#0x1; cmp r0,#0x37` range-guards (errnum-1)<=55, `ldrcc r0,[0x1e9584]`(->0x3054f8) loads table base, `ldrcc r9,[r0,r5,lsl #0x2]` indexes table[errnum], `adrcs r9,0x1e9588`=\"Internal Error\" out-of-range fallback; appends a parenthesized detail (`adr r1,0x1e95a4`=\"(\", `adr r1,0x1e95a8`=\")\") when an extra arg is present; final `ldr r0,[0x1e95b0]`(->0x315d6c); `str r5,[r0,#0x0]` stores the errnum into the error-state global." }
  - { type: disassembled, ref: "spec/reference/errors.yaml (full 0..55 table, disassembled from the same ptr-table/pool) + spec/reference/sysvars.yaml (ERRNUM @0x2ef53c, ERRLINE @0x2ef1bc, ERRPRG @0x2ef3d4 — all writable=false)" }
  - { type: hw_verified, ref: "sb-oracle O-T5 / S-T14a (Azahar, SB 3.6.0): ERRNUM/ERRLINE persist into DIRECT mode after an error halt — `A=SQR(-1)` -> ERRNUM=10 ERRLINE=1; X=ABS() -> 4; A=1/0 -> 7; S$=5 -> 8. Command-form RUN/CONT inside a program -> errnum 3 (cont.yaml/run.yaml batch 2026-06-23)." }
confidence: disassembled
related:
  - ERRNUM
  - ERRLINE
  - ERRPRG
  - STOP
  - CONT
  - END
  - BREAK
  - RUN
---

# Error model

The contract for how SmileBASIC 3.6.0 raises, reports, and recovers from runtime errors — the
shared substrate behind M1's VM error path (M1-T13) and the oracle's error capture (O-T5).
The one fact that shapes everything else: **SmileBASIC has no error trapping.** There is no
`ON ERROR`, no `TRY`/`CATCH`, no `RESUME`, no error-handler vector. Any runtime error
**halts** the running program on the spot and drops to DIRECT mode; the only recovery is the
DIRECT-mode `CONT` command (and only sometimes — see below). Programs cannot observe or
intercept an error mid-run; they can only *read the residue* (`ERRNUM`/`ERRLINE`/`ERRPRG`)
after the fact, from DIRECT mode.

## The errnum table

Every error is a small integer **errnum** in `0..55`. The canonical, disassembled list lives
in [`spec/reference/errors.yaml`](../reference/errors.yaml) — that table is the authority for
codes, names, and descriptions; this model only describes the machinery around it.

| Range | Codes | Origin | In docs? |
|---|---|---|---|
| Internal | 0, 1, 2 | interpreter-internal (No Error / Internal Error / Illegal Instruction) | no |
| User-facing | 3..47 | the documented error table | yes |
| Extended | 48..55 | binary-only (uninitialized var, protected resource/file, DLC, incompatible stmt, END without call, array too large, too many arguments) | no |

The errnum→message lookup is a single function, **`FUN_001e94a8` @0x1e94a8**:

```
sub r0,r0,#0x1 ; cmp r0,#0x37     ; range-guard (errnum-1) <= 0x37 (=55)
ldrcc r0,[0x1e9584]               ; -> 0x3054f8  (table base, .data)
ldrcc r9,[r0,r5,lsl #0x2]         ; r9 = table[errnum]  (word ptr into the .rodata pool)
adrcs r9,0x1e9588                 ; out-of-range -> "Internal Error" fallback
...                               ; appends " (<detail>)" when an extra arg is present
ldr r0,[0x1e95b0] ; str r5,[r0,#0] ; store errnum (r5) into the error-state global *[0x315d6c]
```

- The table at **`0x3054f8`** is 56 word-pointers (errnum 0..55), each into the ASCII string
  pool **`0x2e965c..0x2e9ac0`**. The word just past entry 55 is `0`, marking the table end.
- An errnum outside `1..55` falls back to the literal **"Internal Error"** (`@0x1e9588`) — the
  same string as errnum 1, so an out-of-range code is indistinguishable from a genuine internal
  error in the displayed message.
- The formatter builds a **`<message> (<detail>)`** form: when given an extra numeric/string
  argument it wraps it in parentheses (the `"("`/`")"` literals at `0x1e95a4`/`0x1e95a8`). This
  is how the on-screen report carries the offending value/symbol alongside the message text.
- Where the binary's wording differs from the docs, `errors.yaml`'s `name` is the **binary**
  string (what SB actually displays); `docs_name` records the docs wording (errnum 41 "String
  is too long" vs docs "String too long"; 43 "Can't use from direct mode" vs "…DIRECT mode").

## Error state: ERRNUM / ERRLINE / ERRPRG

When an error halts a program, three read-only system variables capture *where and what*:

| Sysvar | Type | Meaning | addr |
|---|---|---|---|
| `ERRNUM` | Integer | the errnum that halted the program (0 = none) | `0x2ef53c` |
| `ERRLINE` | Integer | the source line the error occurred on | `0x2ef1bc` |
| `ERRPRG` | Integer | the program SLOT (0..3) the error occurred in | `0x2ef3d4` |

All three are **read-only** (`spec/reference/sysvars.yaml`: `writable=false`) — assigning to
them is a Syntax error, not a write. They form the *only* programmatic window onto an error,
and they are meaningful **after** a halt, read from DIRECT mode.

**Persistence into DIRECT mode (hw_verified, O-T5).** The decisive observed fact: ERRNUM and
ERRLINE survive the halt and are readable at the DIRECT-mode prompt afterward. On real SB
3.6.0:

```
A=SQR(-1)   -> halts; ERRNUM=10 (Out of range), ERRLINE=1
X=ABS()     -> halts; ERRNUM=4  (Illegal function call)
A=1/0       -> halts; ERRNUM=7  (Divide by zero)
S$=5        -> halts; ERRNUM=8  (Type mismatch)
```

This is exactly what the oracle relies on: SB has no trap, so the harness runs a statement
expected to fail, lets it halt, and reads `ERRNUM`/`ERRLINE` back in DIRECT mode (see
`run_case.py errcase` / `|err`). The interpreter must clear ERRNUM to 0 (errnum 0 = "No Error")
on a clean run/`ACLS`/`CLEAR`-class reset and set the trio at the moment of the halt; `ERRPRG`
records which of the 4 slots was executing (cross-slot `GOSUB`/`CALL` means the halting line may
be in a different slot than `RUN` started).

## Halt vs. exit vs. break vs. suspend — four different stops

These are distinct and must not be conflated:

| Construct | What it does | Resumable? | Error state |
|---|---|---|---|
| **runtime error** | aborts the program immediately, drops to DIRECT | only via `CONT`, conditionally | sets ERRNUM/ERRLINE/ERRPRG |
| **`END`** | normal program exit (form 1); or closes a `DEF` body (form 2) | n/a — finished cleanly | leaves ERRNUM = 0 |
| **`STOP`** | *suspends* a running program (debug pause), shows `SLOT:line` | yes, via `CONT` | not an error (ERRNUM unchanged) |
| **`BREAK`** | exits the innermost `FOR`/`WHILE`/`REPEAT` loop only | continues after the loop | not an error |
| **START button** | user interrupt → suspends like `STOP` | yes, via `CONT` | not an error |

`END` form 2 (closing a `DEF`) is a parser/structural use, unrelated to program termination —
the two share the keyword but the compiler resolves which by context (inside a `DEF` body it
closes the definition; at top level it ends the program). `BREAK` only unwinds one loop level;
it is not a program stop.

## Suspend / resume (`STOP` ↔ `CONT`)

A program enters the **suspended** state from three triggers (docs, `cont.md`): the **START
button**, the **`STOP`** instruction, or **an error**. On suspend, SB displays the suspended
location as **`SLOT:line`** (`stop.md`) and returns to the DIRECT-mode prompt. **`CONT`**
(DIRECT-mode only) resumes execution *from the suspension point*.

Resume is **not always possible** (`cont.md`). It fails when:

- the program was **edited** after stopping (the compiled code / line map no longer matches the
  saved resume point),
- the program was suspended **while waiting for user input**, or
- **the error type** doesn't allow it (some errors leave unrecoverable state).

A `CONT` that can't resume raises **errnum 33 "Can't continue"** (`spec/reference/errors.yaml`).
`CONT`/`RUN` themselves are **DIRECT-mode command keywords**, *not* program-mode builtins and
*not* reserved identifiers: inside a program `CONT`/`RUN` parse as ordinary variables
(`CONT=2` assigns), and a command-form `CONT`/`RUN slot` statement in a program is a **Syntax
error (errnum 3)** — *not* errnum 44 "Can't use in program" (hw_verified, see
`spec/instructions/cont.yaml` + `run.yaml`). Their resume/launch handlers are index-dispatched
by the command-line interpreter and were not body-pinned, so the exact resume algorithm (and
the precise set of "resumable" error types) is `hypothesis` and queued for the oracle.

## DIRECT-mode error context

DIRECT mode is where errors surface and where the error state is read. Two DIRECT-specific
error codes guard the program/DIRECT boundary:

- **errnum 43 "Can't use from direct mode"** — an instruction that only works inside a program
  was typed at the DIRECT prompt.
- **errnum 44 "Can't use in program"** — a DIRECT-only instruction was used inside a program.

Note the asymmetry confirmed on the oracle: command keywords like `RUN`/`CONT` are *parsed as
variables* in a program, so the failing form is the generic **Syntax error (3)**, not 44 — 44
is reserved for instructions that *are* real builtins but are DIRECT-only.

## Implementation contract (M1-T13)

- Represent an error as `(errnum: i32, errline: i32, errprg: i32)`; raising it **unwinds the
  whole call stack** to the top (no handler search — there is none) and stops the program.
- On raise: set ERRNUM/ERRLINE/ERRPRG, leave them readable from the DIRECT context, and (in the
  display/console path) format `<message> (<detail>)` via the errnum table for the on-screen
  report. The message strings come from `spec/reference/errors.yaml` (display the **binary**
  `name`, not `docs_name`).
- `END` terminates cleanly with ERRNUM = 0; `STOP`/START suspend with the resume point recorded
  but no error set; `BREAK` is loop-local control flow, never touches error state.
- Clear ERRNUM to 0 on `ACLS`/`CLEAR`/fresh `RUN` so a later read isn't contaminated by a prior
  run's error.
- The deterministic gate verifies error *codes* (`|err` fixtures → ERRNUM/ERRLINE), not the
  on-screen formatted string; the formatter's exact `(detail)` content per errnum is queued.

## Corpus notes (real-program usage)

- `ERRNUM` / `ERRLINE` / `ERRPRG` appear in real programs as **read-only diagnostics**, almost
  always printed after a stop — e.g. `PRINT " ERRNUM :";ERRNUM` (`A3X3834J`-class debug
  harnesses) and in sysvar-name `DATA` lists used by editors/tools. `ERRLINE` in ~8 programs,
  `ERRPRG` in ~7 — confirming both are live, read-only sysvars, never assigned.
- `STOP` is common (~342 programs) as a deliberate debug-suspend point. *(community confidence
  — the corpus proves these names resolve as the read-only sysvars / the STOP keyword; it does
  not prove output.)*

## Open questions → oracle (tracked in beads — bd search "oracle")

- **Exact "resumable error" set.** Which errnums leave a CONT-able state vs. force errnum 33
  "Can't continue"? Docs only say "depending on the error type." Pin per-errnum on the oracle.
- **ERRPRG after a cross-slot halt.** Confirm ERRPRG = the slot the *halting line* lives in
  (not the slot `RUN` started) when the error fires inside a cross-slot `GOSUB`/`CALL`.
- **ERRNUM clear points.** Exactly which operations zero ERRNUM (`ACLS`, `CLEAR`, `RUN`, `NEW`,
  a clean `END`?) — verify on the oracle; assumed `ACLS`/`CLEAR`/fresh `RUN`.
- **The formatted `(detail)` per errnum.** The formatter appends a parenthesized value/symbol
  for some errors — harvest the exact on-screen text per errnum (display-only, not a value
  golden).
- **STOP/START suspend display** — confirm the literal `SLOT:line` format string and whether it
  matches the error-halt display.
- **errnum 1 vs out-of-range** — both render "Internal Error"; confirm there is no other
  user-visible distinction.
