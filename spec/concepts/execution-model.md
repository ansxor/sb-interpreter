---
title: Execution model
slug: execution-model
area: execution
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/var.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/def.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/common.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/use.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/exec.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/option.md" }
  - { type: community, ref: "osb VM.d / compiler.d / parser.d — structural reference only (D, 3.5.0); behavior confirmed vs docs/disasm where it matters", confidence: hypothesis }
  - { type: disassembled, ref: "frame layout (currentFunction, old bp, return addr) + slot/COMMON dispatch NOT yet read from cia_3.6.0.lst", confidence: hypothesis }
confidence: documented
related: [VAR, DIM, DEF, COMMON, USE, EXEC, OPTION, GOTO, GOSUB, RETURN, CALL, ON, DATA, READ, RESTORE]
---

# Execution model

The contract for the `sb-core` pipeline (M1): how SmileBASIC source becomes a running
program. The pipeline is

```
source text → lexer → parser (+ const-fold) → compiler → bytecode → stack VM
```

all inside `sb-core` (no I/O / GUI / threads; must build for wasm32 — see prd/M1.md).
Integer = **i32**, Double = **f64** (match SmileBASIC, not Rust/osb). osb is a STRUCTURAL
reference only; where it disagrees with the docs/disassembly, the docs/disassembly win.

## Program slots

SmileBASIC has **4 program slots, `0`–`3`** (resource names `PRG0`–`PRG3`), plus a
transient **DIRECT mode** context for the command line. Each slot is an *independent*
program with its own:

| Per-slot state | Notes |
| --- | --- |
| compiled code + source-location table | one bytecode `Vec<Op>` |
| global variables + global symbol table | name → slot index |
| user functions (`DEF`) | name → `Function` |
| labels (`@name`) | name → code address |
| `DATA` table | flat constant pool for `READ`/`RESTORE` |

- `USE n` (`use.md`) makes the program in slot *n* executable (compiles/links it) so its
  defs/labels are callable.
- `EXEC "[PRGn:]file"` (`exec.md`) loads a program into a slot and runs it; you cannot
  *return* from a plain `EXEC`, but a program `EXEC`'d **into another slot** can `END` back.
  `EXEC` is illegal in DIRECT mode.
- Cross-slot control transfer uses a slot prefix: `GOSUB "0:@SUB"`, `GOTO "1:@LBL"`. A
  return address therefore carries **both** a slot number and a code address (see Frame
  layout). Plain (unprefixed) `GOTO`/`GOSUB` stay within the current slot.

### COMMON / DEF scope

- `DEF`…`END` defines a **user instruction** (`def.md`). Variables and labels inside a
  `DEF` body are **local**; `GOTO` out of the body is forbidden, and `GOSUB`/`ON…GOSUB`
  inside a body is forbidden *unless* it names a slot (`GOSUB "0:@SUB"`).
- Argument/return **types are not checked strictly** on a `DEF` call; a numeric return is
  Integer or Double depending on the value written to the OUT variable (`A=100` → Integer,
  `A=100.0` → Double). A type mismatch *at the receiver* raises Type mismatch (errnum 8).
- `COMMON DEF` (`common.md`) publishes a function into a process-wide **common function
  table** shared by all slots, so a unique instruction defined in one slot is callable from
  another. Re-loading a slot unpublishes its commons first; publishing a name that already
  exists raises a duplicate-function error.

## Source text & lexing

- Source is **UTF-16** text. ⚠ Identifiers are **not ASCII-only**: SmileBASIC is Japanese
  and accepts full-width / kana names. The docs (`var.md`) say "alphanumeric + underscore",
  but that is a doc simplification — confirm the real identifier class (full-width/kana,
  leading-digit rule) vs disassembly/oracle (queued; this is exactly osb's `lexer` limit we
  must NOT inherit, per prd/M1.md).
- **Type by suffix** on a name: none/`%` → numeric, `#` → real, `$` → string. Precisely:
  - `$` → **String**.
  - `%` → **Integer** (i32).
  - `#` → **Real / Double** (f64).
  - **no suffix → dynamically typed numeric**: the variable holds whatever numeric value is
    assigned (Integer or Double), and `DEF` returns pick type from the assigned value
    (`def.md`). It is *not* "always real". (Confirm the exact promotion rule on mixed
    reassignment vs oracle — queued.)
- Literals: decimal + `.`-leading reals, `&H…` hex, `&B…` binary, string literals (SB
  tolerates an unterminated `"…` to end-of-line), `@label`, `#const`. `TRUE`/`FALSE` lex to
  Integer `1`/`0`. Comments: `'` and `REM` to end-of-line.
- Two-char operators: `==  !=  <=  >=  <<  >>  &&  ||`. Statement separator `:`; line
  numbers / `SourceLocation` track across both `:` and newlines.

## Parsing & operator precedence

Expressions parse by **precedence climbing**. Ranks (higher binds *looser*; from osb
`parser.d getOPRank`, structural — the authoritative ranks are pinned by the parser spec /
M1-T3 + oracle):

```
rank  operators
 11   ||
 10   &&
  9   OR   XOR
  7   AND
  6   ==  !=  <  <=  >  >=
  5   <<  >>
  4   +  -          (binary)
  3   *  /  DIV  MOD
  2   [ ]           (array index)
```

Unary `-`, `NOT`, `!` and parenthesised groups bind tighter than rank 2. (`^` power: not in
osb's `getOPRank`; confirm its rank + right-associativity vs oracle — queued.)

**Constant folding at parse time**: a constant-`op`-constant subexpression and unary minus
on a constant are folded during parse (osb `constcalc` / `calc`). The fold must use the
*runtime* semantics (i32 wrap for Integer ops, f64 for real) so a folded constant equals the
value the VM would compute — otherwise a divide-by-zero in dead constant code, integer
overflow wrap, etc. would differ. Malformed input raises Syntax error (errnum 3).

## Compilation

The compiler walks the AST to a flat opcode list (M1 uses `enum Op` + `Vec<Op>` with a
`match` dispatch loop — better for Rust/wasm/determinism than osb's object-per-opcode `Code`
subclasses; opcode *semantics* stay faithful). Responsibilities:

- **Variable resolution** — globals get a slot-global index; `DEF`-locals get a
  **bp-relative** index. `OPTION STRICT` (`option.md`) requires every variable be declared
  (`VAR`/`DIM`) first; without it, first use auto-declares. `OPTION DEFINT` makes
  suffix-less numerics default to Integer.
- **Labels** — two-pass so forward `@labels` resolve.
- **Functions** — `DEF`/`COMMON DEF` bodies compile to addressed `Function`s with an arg
  count, OUT-arg count, and a local-variable frame size.
- **DATA** — all `DATA` items flatten into the slot's `DATA` table; `READ` walks a cursor,
  `RESTORE @label` repositions it.

Opcode families (semantics mirror osb `VM.d`'s `CodeType`): push const / push global / push
local / operate(op) / jumps + conditional jumps + computed `ON` goto / gosub + return /
array new+index+store / call user fn / call builtin / read+restore / print / end.

## The stack VM

A **stack machine** with an operand stack, a frame base pointer `bp`, a program counter
`pc`, and the current slot index. State lives per-slot (4 slots) plus the shared common
function table; the *running* `bp`/`pc`/`stack` are VM-global and swap slot on a cross-slot
call/return.

### Frame layout (per `DEF` / function call)

On a user-function call the VM, with `bp` set to the current stack top, pushes a **3-cell
frame** then the locals (osb `compiler.d frameSize = 3`, `VM.d call()`):

```
stack (grows up)                 bp set to old stacki
  ...
  [ caller's currentFunction ]   ← frame cell 0   (saved enclosing fn)
  [ caller's bp              ]   ← frame cell 1   (saved base pointer)
  [ return address          ]   ← frame cell 2   (slot# + pc, "InternalSlotAddress")
  [ local var 1             ]   ← bp + frameSize + 0
  [ local var 2             ]   ...
```

then `bp ← old stacki`, `pc ← func.address`, and `stacki` advances past the locals (each
local pre-initialised to its declared type's zero). **Return** restores `stacki = bp +
frameSize`, pops the return address (which may switch slot), pops the saved `bp` and saved
`currentFunction`, drops the args, then pushes the return value (or, for an OUT-style call,
copies the OUT locals back). `GOSUB` pushes only a return address (no new `bp`/locals);
`RETURN` pops it. The return address being a *slot+address* pair is what makes cross-slot
`GOSUB "n:@L"` / `RETURN` work.

⚠ The exact 3.6.0 frame cell order, the args-vs-locals overlap, and `RETURN`'s OUT-copy
offsets are taken from osb (3.5.0) structurally and are **not yet confirmed against
`cia_3.6.0.lst`**. They must be read from the disassembly (and differentially checked vs the
oracle) before M1-T6 calls them `hw_verified` — queued in beads (bd:sb-interpreter-air).

### Errors

A runtime error raises an `ERRNUM` and records `ERRLINE`/`ERRPRG` (see
`spec/reference/errors.yaml` + the error-model concept spec, S-C7). Relevant to this model:
Syntax error 3 (parse), Illegal function call 4 (bad arg/out count to a `DEF`/builtin),
Stack overflow 5 (recursion / operand-stack exhaustion), Type mismatch 8 (`DEF`
return/receiver type clash), Subscript out of range 31 (array index). SmileBASIC has **no
error trapping** — an error halts the program; `ERRNUM`/`ERRLINE` persist into DIRECT mode
(verified, O-T5).

## Open (→ disassembly + oracle)

- Real identifier class (full-width/kana, leading-digit rule) — confirm vs oracle.
- Suffix-less numeric promotion rule on mixed Integer/Double reassignment.
- `^` (power) precedence rank + associativity.
- Exact 3.6.0 call-frame cell order / args-locals overlap / `RETURN` OUT-copy offsets —
  read from `cia_3.6.0.lst`, differentially check vs oracle.
- Operand-stack size / recursion depth that trips Stack overflow (errnum 5).
