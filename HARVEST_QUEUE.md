# Harvest queue — behaviors needing oracle (Citra) verification

The autonomous Ralph loop **cannot** run the emulator (the Citra/Azahar oracle is offline/
manual by design — see `harness/README.md`). So when the loop implements a behavior it
can't pin down from the docs or disassembly, it records the open question here instead of
silently guessing. A maintainer later resolves these via `harness/harvest/` and freezes the
answer into a `spec/tests/<id>.yaml` overlay (`confidence: hw_verified`), then deletes the line.

Format: `- [ ] <task/id> · <question> · assumption: <what the code currently does>`

## Open

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
- [ ] S-T4e DTREAD errnum-10 trigger · The handler has an out-of-range branch (errnum 10
  @0x146174) but `DTREAD "2014/13/12"` is ACCEPTED (oracle 2026-06-22, no error). · find what
  DOES trigger it (e.g. impossible day like "2014/02/30", or a non-Gregorian/zero date) so the
  errnum-10 condition can be specced. assumption: it's a day-vs-month-length check, not a month
  range. DTREAD value/weekday/format/type-mismatch cases already hw_verified 2026-06-22.
- [ ] S-T4e slot-prefix + CHKLABEL flag · Confirm `CHKCALL("0:CHR$")`/`CHKVAR("N:VAR")`/
  `CHKLABEL("1:@L")` (slot prefix) and bad-slot → FALSE (not error); confirm CHKLABEL flag=1
  searches global labels when not found inside a DEF, flag=0 restricts to the DEF. · assumption
  (from disasm @0x28e5c0 + osb VM.d): slot prefix routes the lookup to slot N, invalid slot
  yields FALSE; flag is boolean. Needs a multi-slot/USE + DEF oracle setup. Base CHK* true/false
  + non-string-errnum-8 cases already hw_verified 2026-06-22.
- [ ] S-T4f VSYNC/WAIT timing semantics · Confirm VSYNC counts from the last VSYNC and WAIT
  from the present (e.g. via MAINCNT deltas across a known-cost loop) and that count<=0 is a
  no-op that resyncs lastVsync. · assumption (disasm @0x1455c8/@0x14afb0): VSYNC target =
  lastVsync+n, WAIT target = current+n, both set lastVsync=current on exit. Needs the M4 frame
  clock + a deterministic MAINCNT probe. errnum-4 (used as function) already hw_verified 2026-06-22.
- [ ] S-T4f KEY() function form · Confirm `KEY 3,"HI":PRINT KEY(3)` returns the bound string
  "HI", and that KEY(n) honors the 1..5 range (errnum 10 out of range). · assumption (disasm
  @0x14c018 retcount==1 path + corpus VAL(KEY(5))): KEY(n) reads back the assigned function-key
  string. Statement errnum 8/10 cases already hw_verified 2026-06-22.
- [ ] S-T4f OPTION STRICT/DEFINT behavior + OPTION TOOL · Confirm OPTION STRICT makes an
  undeclared reference raise (errnum 15 Undefined variable assumed) and is position-dependent;
  OPTION DEFINT makes unsuffixed vars Integer; what OPTION TOOL (12 corpus programs) does at
  compile time. · assumption: STRICT undeclared -> errnum 15; DEFINT default int. Unknown-feature
  errnum 3 already hw_verified 2026-06-22.
- [ ] S-T4f DIALOG interactive forms + RESULT · Confirm R=DIALOG(text,seltype,...) returns
  -1/0/1, button-detect (negative mask) returns 128..140, the file-name form returns the entered
  string with RESULT=-1 on cancel, and the colon-prefixed menu string with seltype -1 behavior.
  · BLOCKS on Touch-Screen input — not harvestable with the current headless oracle (no input
  injection). assumption (docs + disasm @0x181050): per the documented return tables. Argcount>4
  -> errnum 3 already hw_verified 2026-06-22.
- [ ] S-T5a PRINT console-output cases · The value-oracle (run_case.py batch) captures VALUE
  results, not console text, so PRINT stdout (e.g. `PRINT "HI"` -> "HI", `PRINT "A";"B"` -> "AB",
  `,` tab-advance via TABSTEP, trailing-separator newline suppression) is not harvestable through
  it. Needs the screenshot/console-grid path. · assumption (docs + disasm @0x14b70c): items
  printed left-to-right; `;` no gap; `,` to next TABSTEP stop; trailing `;`/`,` suppresses the
  line break. errnum 8 (Type mismatch on a non-printable operand) also unharvested — hard to
  construct a non-printable PRINT operand from the value path.
- [ ] S-T5a LOCATE Z exact bounds · Confirm the Z (depth) lower bound -256.0 and upper bound
  1024.0 precisely (only Z=2000 -> errnum 10 is hw_verified so far). · assumption (disasm
  @0x14bce0 float constants 0xC3800000 = -256.0, 0x44800000 = 1024.0, inclusive): out raises
  errnum 10.
- [ ] S-T5b INPUT/LINPUT array-element receivers · Confirm `INPUT "...";WORD$[0]` and
  `LINPUT NAMES$[0]` runtime-assign into array elements (syntax proven by corpus:
  D5243E8E/TXT/TXTDEMO:41, E3S34XGX/TXT/BATTLESHIP:84). · assumption (disasm @0x14b534 lvalue
  tag 8/9 check): array elements are accepted lvalues. Needs live keyboard so output
  oracle-pending (INPUT/LINPUT block).
- [ ] S-T5b INPUT read/assign + "?Redo from start" · The actual read-line, comma-field-split,
  numeric parse, and insufficient-items re-input loop are unharvested (INPUT blocks on the
  keyboard; oracle has no input redirection). · assumption (docs + disasm @0x14b5a4 read line,
  @0x14b5b8 field parse): line split on commas, redo on shortage.
- [ ] S-T5b INKEY$ live keypress · INKEY$() returning an actual queued char is unharvested
  (real-time keyboard); only the empty-buffer "" is hw_verified. · assumption (disasm @0x14b234
  strh of one UTF-16 unit): returns a 1-char string of the popped key.
- [x] S-T5c value/errnum cases · HARVESTED 2026-06-22 (sb-oracle batch s_t5c → spec hw_verified):
  ATTR 16/-1 → errnum 10, A=ATTR(3) → errnum 4; CHKCHR(0,0)=65 after PRINT "A", CHKCHR(-1,0)=0,
  CHKCHR(0,100)=0, A=CHKCHR(0)/CHKCHR 0,0 → errnum 4; FONTDEF 70000/-1 → errnum 10, bad-hex
  → errnum 4, short array → errnum 31, A=FONTDEF(...) → errnum 4; WIDTH()=8 default / 16 after
  WIDTH 16, WIDTH 12/0 → errnum 4 (NOT 10); SCROLL 5 / A=SCROLL(5,7) → errnum 4.
- [ ] S-T5c visual side-effects (screenshot path) · ATTR rotation/inversion render, FONTDEF glyph
  pixels, SCROLL pixel movement, WIDTH 8↔16 reflow are not VALUE-harvestable — they need the
  framebuffer/screenshot oracle (not yet in the skill). Behavior is from docs + disassembly.

- [x] S-T7a errnum cases · HARVESTED 2026-06-22 (sb-oracle batch s_t7a → specs hw_verified):
  GPAGE 6,0 / 0,-1 → errnum 10, GPAGE 0 (1 arg) → errnum 4, **GPAGE 0,0,0 (3-arg corpus form)
  → errnum 4** (disasm confirmed — strict 2-arg). GCLS() → errnum 4, GCLS 0,0 → errnum 4.
  GPRIO 1025 / -257 → errnum 10 (confirms -256..1024 range), GPRIO(0) → errnum 4. GCLIP 0,1,2
  → errnum 4, GCLIP(0) → errnum 4. GCOLOR (no arg) → errnum 4, GCOLOR 1,2 → errnum 4.
- [ ] S-T7a remaining round-trip values (not value-batchable — need setup-then-PRINT program):
  GPAGE 1,2 → OUT V,W = 1,2; GCOLOR 100 → OUT C / C=GCOLOR() = 100; GCLIP write-mode bad
  rectangle → errnum 10 (which region triggers it?). These are disassembled-solid (store/load)
  but not yet oracle-confirmed.
- [ ] S-T7a visual side-effects (framebuffer path) · GCLS fill color, GCOLOR draw color applied
  to primitives, GCLIP clip region, GPRIO layer Z-order, GPAGE display/draw page selection are
  pixel effects — need the framebuffer oracle (O-T6, not yet in the skill). Behavior is from
  docs + disassembly.

- [x] S-T7b errnum cases · HARVESTED 2026-06-22 (sb-oracle batch s_t7b → specs hw_verified):
  all 15 arg-count guards confirmed errnum 4 / errline 1 — GPSET 100 / 1,2,3,4 / A=GPSET(1,1);
  GLINE 0,0,1 / 0,0,1,1,2,3 / A=GLINE(...); GBOX 0,0,1 / 0,0,1,1,2,3 / A=GBOX(...);
  GTRI 0,0,1,1,2 / 0,0,1,1,2,2,3,4 / A=GTRI(...); GCIRCLE 100,100 / 1,1,1,0,45,1,2,3 /
  A=GCIRCLE(100,100,30). Matches the disasm guards (errnum 4 sites @0x153dd0/@0x153318/
  @0x15514c/@0x1554e0/@0x154a80).
- [ ] S-T7b visual side-effects (framebuffer path) · the actual pixels GPSET/GLINE/GBOX/GTRI/
  GCIRCLE draw, the default-color path (current GCOLOR), float-coordinate rounding, GCIRCLE arc
  vs sector geometry + angle normalization (negative / >360 wrap, where 0deg points), and
  radius<=0 no-op are pixel effects — need the framebuffer oracle (O-T6, not yet in the skill).
  Behavior is from docs + disassembly + corpus.

- [x] S-T7c arg-count guards · HARVESTED 2026-06-22 (sb-oracle batch s_t7c → specs hw_verified):
  all 9 confirmed errnum 4 / errline 1 — GFILL 0,0,1 / 0,0,1,1,2,3 / A=GFILL(0,0,1,1);
  GPAINT 200 / 0,0,1,2,3 / A=GPAINT(0,0); GPUTCHR 10,10 / 10,10,"A",2,2,0,0 / A=GPUTCHR(10,10,"A").
  Matches the disasm guards (GFILL @0x153154, GPAINT @0x154544, GPUTCHR @0x154b40 / @0x154c18).
- [ ] S-T7c GPUTCHR float-scale coercion · does a float scale (1.5,1.5) truncate to 1 (no
  scaling) or round? corpus shows ~41 real uses (e.g. 43Y5P31D/TXT/CAR '...,1.5,1.5,...').
  Assumption: integer-truncated by the int arg-fetch (disasm fetches scale via int vtable
  [r2,#0x40] @0x154c6c/@0x154cb8), so 1.5 → 1. Needs oracle + framebuffer to confirm.
- [ ] S-T7c GPUTCHR errnum 49 · the graphic-plane availability guard (mov r0,#0x31 @0x154da4)
  raises errnum 49 — confirm the exact error NAME and the precise state that triggers it
  (plane not displayed/allocated). Not in errors.yaml (which stops at 47); oracle-pending.
- [ ] S-T7c visual side-effects (framebuffer path) · GFILL solid-rect span + default-color
  path, GPAINT flood-fill region (border-omitted = start-point color region vs explicit
  border), GPUTCHR glyph rendering/positioning/scale/font — all pixel effects needing the
  framebuffer oracle (O-T6, not yet in the skill). Behavior is from docs + disassembly + corpus.
- [x] S-T7d arg-count + page-range guards · HARVESTED 2026-06-22 (sb-oracle batch s_t7d → specs hw_verified):
  all 10 confirmed — GCOPY 6args/9args/A=GCOPY(...) → errnum 4, GCOPY 6,... (src page>5) → errnum 10;
  GLOAD W,1 (2 args)/0,0,8,8,W (5 args)/A=GLOAD(W,1,0) → errnum 4; GSAVE 0,0,W,1 (4 args)/A=GSAVE(W,1)
  → errnum 4, GSAVE 6,W,1 (src page>5) → errnum 10. Matches the disasm guards (GCOPY @0x152f00/@0x152f78,
  GLOAD @0x153580, GSAVE @0x153f14/@0x153f78).
- [ ] S-T7d errnum 49 page-availability guard · GCOPY (mov r0,#0x31 @0x1530f0) and GSAVE (@0x154294)
  raise errnum 49 when the resolved source plane is unusable (guard byte [page+0x60] set) — confirm the
  exact error NAME and the precise triggering state. Not in errors.yaml (stops at 47); oracle-pending.
- [ ] S-T7d GLOAD/GSAVE error edges · GLOAD with too-small image_array → errnum 31 (disasm @0x15381c);
  GLOAD non-numeric image_array → errnum 8 (@0x1539a8); GSAVE multi-dim too-small dest_array → errnum 31
  (@0x154218); negative Width/Height → errnum 10 (GSAVE @0x154108, GLOAD @0x153728). Disassembled, oracle-pending.
- [ ] S-T7d visual/array side-effects (framebuffer path) · GCOPY page-to-page blit (transparent copy mode
  on/off), GSAVE pixel→array element format (convert flag 0 = 32-bit logical, 1 = 16-bit physical) + 1-D
  auto-expand to width*height, GLOAD array→page restore (flag vs palette form). All need the framebuffer
  oracle (O-T6, not yet in the skill). Behavior is from docs + disassembly + corpus.
