# Harvest queue — behaviors needing oracle (Citra) verification

The autonomous Ralph loop **cannot** run the emulator (the Citra/Azahar oracle is offline/
manual by design — see `harness/README.md`). So when the loop implements a behavior it
can't pin down from the docs or disassembly, it records the open question here instead of
silently guessing. A maintainer later resolves these via `harness/harvest/` and freezes the
answer into a `spec/tests/<id>.yaml` overlay (`confidence: hw_verified`), then deletes the line.

Format: `- [ ] <task/id> · <question> · assumption: <what the code currently does>`

## Open

- [ ] M1-T1 (lexer) · Does SmileBASIC 3.6.0 allow full-width / kana characters in
  identifiers and labels? · assumption: ASCII-only (inherited from osb — **likely wrong**,
  SB is a Japanese product; verify and fix the lexer's `is_alpha`/identifier scan).
- [ ] M1 (general) · Exact `STR$`/PRINT double→string formatting (sig figs, exponent
  threshold, trailing zeros) · assumption: best-effort Rust formatting (see M7-T4).
  PARTIAL EVIDENCE (oracle 2026-06-21, S-T1): doubles print to **6 significant figures**
  with trailing zeros trimmed — PI()=3.14159, RAD(45)=0.785398, ACOS(0)=1.5708 (PI/2 → 1.57080
  → "1.5708"), COSH(1)=1.54308, EXP(2)=7.38906. Confirm the exponent threshold + rounding mode.

### S-T1 Mathematics (errnums need O-T5 error capture; type/edge cases)
- [ ] S-T1 · Exact errnum for math out-of-range domains (ASIN/ACOS |x|>1, SQR(<0), LOG(<=0)
  and LOG base domain) · assumption: errnum 10 (Out of range), from osb's `OutOfRange`.
- [ ] S-T1 · Exact errnum for passing a string to a math function · assumption: errnum 8
  (Type mismatch), matching the FLOOR exemplar.
- [ ] S-T1 · MAX/MIN result type: does the 2-arg form return Integer when both args are Integer
  while the 3+-value and array forms return Double? · assumption: yes (osb cross-check; osb
  comment "MAX(2,0)*&H7FFFFFFF != MAX(2,0,0)*&H7FFFFFFF" implies the int/double split).
- [ ] S-T1 · RND(max) when max <= 0, and RND/RNDF/RANDOMIZE when seed_id is outside 0-7 ·
  assumption: Out of range (errnum 10) for bad seed_id; max<=0 behavior unconfirmed.
- [ ] S-T1 · ABS(-2147483648) (i32 INT_MIN) — overflow/wrap or promote to Double? · assumption:
  unconfirmed; document once O-T5/value capture can show the type.
- [ ] M1-T1 (lexer) · Is `1E5` lexed as `1` + ident `E5` (no exponent literal)? · assumption:
  yes (osb behavior) — confirm against 3.6.0.
