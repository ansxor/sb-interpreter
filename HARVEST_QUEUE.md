# Harvest queue — behaviors needing oracle (Citra) verification

The autonomous Ralph loop **cannot** run the emulator (the Citra/Azahar oracle is offline/
manual by design — see `harness/README.md`). So when the loop implements a behavior it
can't pin down from the docs or disassembly, it records the open question here instead of
silently guessing. A maintainer later resolves these via `harness/harvest/` and freezes the
answer into a `spec/tests/<id>.yaml` overlay (`confidence: hw_verified`), then deletes the line.

Format: `- [ ] <task/id> · <question> · assumption: <what the code currently does>`

## Open

- [ ] S-T7a (GPRIO/GCLIP/GCLS colors) · Visual / O-T6 golden — verify via the grp/screenshot
  oracle: GPRIO actually reorders the graphics layer vs console/sprite/BG; GCLIP display-mode
  (mode 0) restricts the shown region; the EXACT RGBA5551 round-trip of GCLS/GCOLOR colors
  (e.g. is white 0xFFF8F8F8 or 0xFFFFFFFF?). State read-backs (GPAGE OUT, GCOLOR OUT) + GCLS
  default=0 + GCLIP write-clip behavior already hw_verified 2026-06-22 via GSPOIT.
- [ ] S-T5c (ATTR/FONTDEF) · Visual — verify via screenshot/graphics oracle: ATTR rotation
  (#TROT0-270) and inversion (#TREVH/#TREVV) actually rotate/flip the rendered glyph; FONTDEF
  redefines the on-screen glyph (and the GRP page -1 font image). Also resolve the corpus
  `(ATTR>>8) AND 7` value-read form — bare ATTR returns 0 after a console ATTR set, so what is
  it reading? · assumption: per docs/disasm. ATTR constants, CHKCHR, WIDTH (8/16 + WIDTH()),
  SCROLL direction all hw_verified 2026-06-22.
- [ ] S-T5b (INPUT/LINPUT) · Interactive — block on keyboard, NOT value-harvestable by the
  batch oracle. Verify via screenshot+typed input: INPUT parses the typed line into the var
  list (numeric vs string), ';' shows '?' and ',' suppresses it, string-var prompt with ';',
  "?Redo from start" on too-few values; LINPUT reads a whole line incl. commas into one
  string var (and into a string array element). · assumption: per docs/disasm. INKEY$ empty
  behavior already hw_verified 2026-06-22.
- [ ] S-T4f (DIALOG/KEY/VSYNC/WAIT) · Interactive/timing — NOT value-harvestable by the batch
  oracle. DIALOG is modal (needs a touch/button press; verify RESULT 1/-1/0, the -1/0/1 return,
  the hardware-button 128..140 codes, and negative-timeout=frames via screenshot+input). KEY
  alters the function-key table (and an undocumented KEY(n) FUNCTION read form — confirm it
  returns the assigned string). OPTION TOOL: confirm its runtime effect (undocumented keyword,
  ~12 corpus uses). VSYNC/WAIT: only execution-continuation is hw_verified; the wait DURATION
  (frame timing) needs the frame-and-timing harness (S-C4), not the value oracle. · assumption:
  per docs/disasm. OPTION DEFINT/STRICT behavior already hw_verified 2026-06-22.
- [ ] S-T4d (RESTORE) · Confirm RESTORE to an undefined @Label -> errnum 14 (Undefined label),
  and the cross-slot form RESTORE "1:@Label" after USE 1 (needs a 2nd slot loaded — single-slot
  oracle can't easily test). · assumption: errnum 14 per docs/error-table; cross-slot per docs.
  Core DATA/READ/RESTORE/REM (incl. #const, &H, computed labels, 2D-array READ, out-of-DATA 13,
  type-mismatch 8) all hw_verified 2026-06-22.
- [ ] S-T4c (COPY/FILL/SORT/RSORT) · Confirm the secondary error edges: COPY DATA-form with
  fewer DATA items than required (errnum? docs say "an error occurs" — guess 13 Out of DATA);
  FILL with offset/count beyond array bounds (errnum 31?); SORT/RSORT with bad/missing array
  args (errnum 4?). · assumption: per docs/error-table. Core COPY (incl. 5-arg + DATA form),
  FILL (incl. string+offset), SORT/RSORT (numeric/float/string keys + parallels) all
  hw_verified 2026-06-22.
- [ ] S-T4b (PUSH/UNSHIFT/POP/SHIFT) · Confirm behavior on a MULTI-DIMENSIONAL array (e.g.
  `DIM A[2,2]:PUSH A,9`) — does it error (and which errnum) or operate on a flattened/last
  dimension? · assumption: documented for 1D only; multi-dim likely errors. Corpus shows no
  genuine multi-dim PUSH (only commented-out). 1D + string-as-char-array + empty->errnum 31
  + numeric-scalar->errnum 8 all hw_verified 2026-06-22.
- [ ] S-T4a (VAR/DIM) · Confirm the secondary error edges not harvested in the 2026-06-22
  s_t4a batch: OPTION STRICT + undeclared use -> errnum 15; array size with `()` instead of
  `[]` -> errnum 3; over-large array -> errnum 11 (Out of memory). · assumption: errnums per
  the official error table + docs (var.yaml/dim.yaml `errors:`). Core VAR/DIM/INC/DEC/SWAP
  behavior + duplicate-var (18) + type-mismatch (8) already hw_verified 2026-06-22.
- [ ] M1-T1 (lexer) · Does SmileBASIC 3.6.0 allow full-width / kana characters in
  identifiers and labels? · assumption: ASCII-only (inherited from osb — **likely wrong**,
  SB is a Japanese product; verify and fix the lexer's `is_alpha`/identifier scan).
- [ ] M1 (general) · Exact `STR$`/PRINT double→string formatting (sig figs, exponent
  threshold, trailing zeros) · assumption: best-effort Rust formatting (see M7-T4).
- [ ] M1-T1 (lexer) · Is `1E5` lexed as `1` + ident `E5` (no exponent literal)? · assumption:
  yes (osb behavior) — confirm against 3.6.0.
- [ ] S-T1b (CLASSIFY) · Confirm CLASSIFY returns 1 for infinity and 2 for NaN, and find how
  to *produce* inf/NaN in SB 3.6.0 (no INF/NAN constant; exponent literals doubtful). Try
  arithmetic overflow (e.g. repeated X#=X#*10 from a big value) for inf and SQR(-1)/0.0/0.0
  for NaN — but those may raise errors first. · assumption: inf->1, NaN->2 (disasm: helper
  @0x20c3e0 code 3->1, 7->2). Ordinary (0) + string-error already harvested.
- [ ] S-T1a (FLOOR/ROUND/CEIL) · Do these return a Double for a Double argument (not an
  Integer)? Discriminate with a whole magnitude > i32 max, e.g. `PRINT FLOOR(3000000000.0)` —
  Double return prints the full number, Integer return would overflow (errnum 9) or wrap. ·
  assumption: returns whole-numbered Double (disasm: floor/ceil helpers @0x1ed970/0x1ed760
  are double->double; pushed via Double return path; osb agrees). Also needs STR$ formatting
  resolved (see the M1 STR$ line) to know the exact printed text.
- [ ] S-T1e (PI) · Does supplying an argument to PI (e.g. `A=PI(1)`) raise errnum 4
  (Illegal function call)? PI is niladic; the parser may reject `PI(1)` as a syntax error
  instead. · assumption: errnum 4 (consistent with other math builtins' argcount!=1 check).
  Value cases PI()=3.14159, PI()*2=6.28319 already harvested (hw_verified 2026-06-22).
- [ ] S-T1f (RND/RNDF/RANDOMIZE) · Harvest a deterministic seeded RNG sequence: run
  `RANDOMIZE 0,1` then capture `RND(0,100)` / `RNDF(0)` for several draws, to verify our
  TinyMT128 implementation matches SB's exact sequence. · assumption: TinyMT128 per disasm
  (core @0x1eb598/0x1eac60, seed @0x1ec22c, state table @0x1d08000). Needs RANDOMIZE-then-draw
  sequencing the batch tool's single-expression cases can't express. Error/range cases
  (errnum 4/8/10) + RND(1)=0 already hw_verified 2026-06-22.
- [ ] S-T1f (MIN/MAX) · Harvest the array form `DIM TMP[2]:TMP[0]=50:TMP[1]=3:MIN(TMP)` /
  `MAX(TMP)` and re-capture `MAX(1,"x")` (errnum 8 — capture flaked twice on 2026-06-22). ·
  assumption: array form returns smallest/largest element (disasm @0x148558/0x148230);
  MAX string -> errnum 8 (mirror of the verified MIN case). Varargs values + type preservation
  already hw_verified.
- [ ] S-T2b (VAL) · Re-capture `A=VAL(5)` (non-string -> errnum 8). Oracle ERRNUM capture
  flaked twice on 2026-06-22 ("halted but no read"). · assumption: errnum 8 (disasm @0x148f34,
  and sibling STR$/HEX$/FORMAT$ non-string cases all hw_verified errnum 8 same run).
- [ ] S-T2c (LEN) · Harvest the array form `DIM A[5]:LEN(A)` (-> 5) and a 2-D case. ·
  assumption: returns total element count (disasm @0x147f68 array path vtable +0x5c/+0x14).
  String forms LEN("ABC123")=6 etc. already hw_verified 2026-06-22 (multi-statement DIM not
  batch-harvestable).
- [ ] S-T3a (IF/ENDIF) · Harvest multi-line IF blocks (IF cond THEN <nl> ... ELSE ... ENDIF),
  ENDIF rejoin, ELSE IF (spaced) nesting, and the GOTO-omission form (IF cond THEN @label /
  IF cond GOTO @label). The batch tool's |err cases are single-line (newlines can't be embedded
  in a cases.txt line), so these need a multi-line program harness. · assumption: standard
  block semantics (disasm: keyword table @0x2ed5c8..0x2ed678). Single-line THEN/ELSE/ELSEIF
  branch selection + truthiness already hw_verified 2026-06-22 (error-as-signal).
- [ ] S-T3b (FOR/STEP) · Harvest fractional-STEP iteration counts (e.g. FOR I=0 TO 1 STEP 0.25
  -> how many passes given float error; FOR I=0 TO 2 STEP 0.1) and confirm the loop variable is
  Double in that case. · assumption: standard accumulate-by-step with float drift (documented
  caveat). Integer-step counts/finals/direction + NEXT-without-FOR (errnum 21) already
  hw_verified 2026-06-22.
- [ ] S-T3c (WEND/BREAK) · Live-capture WEND-without-WHILE errnum (expected 25 per
  spec/reference/errors.yaml) and determine BREAK/CONTINUE outside any loop behavior (errnum?
  syntax error? ignored?). Both oracle captures flaked twice on 2026-06-22. · assumption: WEND
  alone -> errnum 25 (table; sibling UNTIL-without-REPEAT=23 confirmed live). BREAK/CONTINUE
  outside a loop: unknown, not in the errors table. Loop behavior + UNTIL/23 already hw_verified.
- [ ] S-T3d (END/STOP/OUT) · Verify END/STOP halt behavior and OUT multi-return. The
  error-as-signal harness can't distinguish a clean halt (END/STOP) from an error-halt because
  the halt also stops the sentinel SAVE (reads stale ERRNUM). Need a stdout-diff harness (e.g.
  PRINT before/after END). OUT needs a DEF context (see S-T3e). · assumption: END halts cleanly
  (not resumable), STOP suspends (CONT-resumable), OUT receives DEF multi-returns -- all from
  docs+disasm (tokens END@0x2ed5a4, STOP@0x2ed598, OUT@0x2ed528). GOTO/GOSUB/RETURN/ON already
  hw_verified 2026-06-22 (incl. errnum 14 undefined label, 30 RETURN-without-GOSUB).
- [ ] S-T3e (XON/XOFF/COMMON) · Harvest XON EXPAD -> RESULT=TRUE (and XON MOTION/MIC), XOFF,
  and COMMON cross-slot visibility. XON may pop a confirmation dialog (could hang the headless
  |err harness) and needs a feature that the emulator supports; COMMON cross-slot needs a
  multi-slot harness. · assumption: XON enables MOTION/EXPAD/MIC (RESULT TRUE on EXPAD success),
  XOFF disables; COMMON DEF is callable from another slot after USE. DEF/CALL + same-slot
  COMMON + errnum 16/29/32 already hw_verified 2026-06-22.

## Corpus-discovered (sbsave grep, 2026-06-22)

Edge cases surfaced by grepping real usage in `harness/corpus/sbsave/` and added to the
specs at `confidence: community` (one `type: community` source line each). They need the
oracle to confirm exact output and promote to `hw_verified`.

- [ ] FORMAT$ (%% / %B) · Confirm `FORMAT$("%D%%",50)`="50%" (literal percent) and what
  `FORMAT$("%04B",10)` produces (is %B a real binary directive, and is it zero-padded like
  %D?). · assumption: %% -> literal '%'; %B -> binary digits. Corpus: %% in 19 programs, %B
  in 7 (e.g. 4K241XVD/TXT/DOTMAGICS-C). %S/%D/%X/%F + flags already hw_verified 2026-06-22.
- [ ] ENDIF (single-line) · Confirm `IF 1 THEN PRINT "A" ENDIF` and
  `IF c THEN a ELSE b ENDIF` run on one line (ENDIF closing a single-line IF). · assumption:
  accepted (output "A"). Corpus: 86 programs (e.g. 1DVK34J/TXT/HNZBUS) — contradicts the old
  "single-line form does not use ENDIF" note. Multi-line ENDIF still queued (S-T3a).
- [ ] GOTO/GOSUB (string-expr target) · Confirm a runtime-built label string branches:
  `L$="@X":GOTO L$` and `GOTO "@LK_"+K$` reach @X / @LK_<k>. · assumption: label string is
  evaluated, then resolved like a literal. Corpus: 82+ programs (GOTO L$, GOTO "0:@TAB"+S$).
- [ ] ON (array/expr index) · Confirm `ON ARR[I] GOSUB ...` and `ON RND(3) GOTO ...` select by
  the evaluated integer. · assumption: index is any int expression (0-based). Corpus: 33
  programs (e.g. 13D4DV3V/TXT/MAIN_PRG_V2). Scalar 0-based + fall-through already hw_verified.
- [ ] RETURN (value form) · Confirm `DEF F():RETURN 7:END` makes `PRINT F()` print 7, and the
  multi-value `RETURN a,b` -> OUT roundtrip. · assumption: RETURN expr hands the value(s) back
  to a DEF caller. Corpus: 1143 programs. Pairs with S-T3d/S-T3e (DEF/OUT). GOSUB-return form
  already hw_verified 2026-06-22.
- [ ] VAL (&B binary) · Confirm `VAL("&B1010")`=10 (and `VAL("&B"+bits$)` parses binary). ·
  assumption: &B is a recognized binary literal prefix like &H. Corpus: 11 programs (e.g.
  13D4DV3V/TXT/MAIN_PRG_V2). &H/exponent/strict-whole-string already hw_verified 2026-06-22.
