# Harvest queue — behaviors needing oracle (Citra) verification

The autonomous Ralph loop **cannot** run the emulator (the Citra/Azahar oracle is offline/
manual by design — see `harness/README.md`). So when the loop implements a behavior it
can't pin down from the docs or disassembly, it records the open question here instead of
silently guessing. A maintainer later resolves these via `harness/harvest/` and freezes the
answer into a `spec/tests/<id>.yaml` overlay (`confidence: hw_verified`), then deletes the line.

Format: `- [ ] <task/id> · <question> · assumption: <what the code currently does>`

## Open

- [ ] M1-T1 (lexer identifier class) · What is the exact SmileBASIC 3.6.0 identifier
  character class — which non-ASCII chars are legal in a name (kana/kanji/full-width latin/
  full-width digits?), and the leading-char rule (can a name start with a digit, `_`, or a
  full-width digit?)? Also confirm names are case-insensitive (ASCII fold to upper) and
  whether full-width letters fold too. · assumption: start = Unicode `is_alphabetic()` or `_`;
  continue = Unicode `is_alphanumeric()` or `_`; ASCII case-folded to upper, non-ASCII left
  as-is. Probe e.g. `Ａ=1:?Ａ` (full-width A), `１A=1` (full-width-digit lead), `あ=1:?あ`.

- [ ] S-T13a (MPSTART/MPEND/MPSET/MPSTAT) · Wireless-session behaviors are body-pinned for
  validation (arg-count errnum 4, MP-restriction errnum 52 "Incompatible statement" via flag
  @0x305612, range errnum 10, MPSTART non-string identifier errnum 8) but the actual NETWORK
  effects need real wireless peers (the single Azahar oracle can't form a 2-4 player session).
  Open questions: (a) is the MP-restriction flag @0x305612 zero in a normal program-mode
  session — i.e. does a valid MPSTART proceed to a connection dialog, or does it immediately
  raise errnum 52? (b) MPSTART RESULT value on success/failure/timeout; (c) MPSTAT 0/1 return
  for self vs peers and whole-session; (d) MPSET Double operand — truncated or errnum 8?;
  (e) does real SB reject the corpus 3-arg `MPSET a,b,c` (C2NVX3QJ/PETITWORLD) with errnum 4 as
  the handler's `cmp r0,#0x2` implies? · assumption: validation errnums per disassembly; network
  results documented-only. NOTE: MPSTART/MPEND attempt real networking — harvest cautiously
  (connection dialogs may hang the oracle); the pre-network validation errors (errnum 4/10/8)
  are the safe cases to harvest first.
- [ ] S-T13b (MPSEND/MPRECV/MPGET/MPNAME$) · Messaging behaviors are body-pinned for validation
  (arg/return-count errnum 4, MP-restriction errnum 52 via flag @0x305612, MPSEND non-string
  errnum 8 / empty-string errnum 4 / >128-codeunit errnum 41 "String too long" / send-overflow
  errnum 42 "Communication buffer overflow", MPGET/MPNAME$ terminal-id & MPGET mgmt-num errnum
  10, MPRECV/MPNAME$ alloc errnum 11) but the actual messaging effects need ≥2 real wireless
  peers (single Azahar oracle can't form a session). Open questions: (a) MPSEND delivery delay +
  the burst rate that triggers errnum 42; (b) MPRECV SID/RCV$ values + the -1 no-data sentinel
  in practice; (c) MPGET returned slot value (peer-set via MPSET) per management number 0-8;
  (d) MPNAME$ returned terminal-name string; (e) CORPUS WORD-ORDER QUESTION: 3 shipped programs
  use `MPRECV SID OUT RCV$` (var before OUT) — 4KY3343D/ANROI-DS+@BACKUP.PRG, Q3XPAE83/
  QUICKTOOL_PLUS, QDH3J37D/ANDROI-DS — printing SID as the sender afterward, yet the handler's
  value-arg-count==0 guard @0x183e98 would reject a value arg with errnum 4. Does 3.6.0 accept
  this alternate parse, or are these latent bugs? · assumption: validation errnums per
  disassembly; messaging payloads documented-only.
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
- [ ] S-T7e color read (GSPOIT · RGB · RGBREAD) · Value/error expects HARVESTED (sb-oracle 2026-06-22 s_t7e):
  GSPOIT off-page -> 0 (NOT -1 as PTC docs claim); RGB clamps channels to 0-255 (RGB(999,999,999)=-1);
  RGB/GSPOIT arg-count errors -> errnum 4. RESOLVED 2026-06-23 (s_c2): (a) GSPOIT post-draw round-trip is
  HARVESTED via the multi-statement prog path (GPSET x,y,RGB(...):GSPOIT(x,y)) — RGB(255,0,0)->-524288,
  RGB(255,255,255)->-460552 (==#WHITE), RGB(0,100,0)->-16752640; folded into GSPOIT.yaml tests (hw_verified)
  and spec/concepts/screen-and-color-model.md. STILL PENDING: (b) RGBREAD value round-trip
  (RGBREAD &HFF804020 OUT R,G,B -> R,G,B) — RGBREAD is a statement writing OUT vars, so the expr/value batch
  can't capture it (prog path returns None for stmt+expr); extraction is fully disassembled (shift+mask) but a
  direct hw_verified anchor is queued. (c) GSPOIT errnum 49 (0x31) graphics-state guard @0x1543bc — undocumented
  (beyond the 3-47 table), not reachable from ordinary user code; confirm trigger if ever possible.
- [x] S-T8a sprite lifecycle (SPSET · SPCLR · SPSHOW · SPHIDE · SPPAGE) · ERROR expects HARVESTED hw_verified
  (sb-oracle 2026-06-22 s_t8a): SPSET 512,0 / -1,0 -> errnum 10; SPSET 0,4096 -> errnum 10; SPSET 0,0,0,0,0,0,0
  -> errnum 4; SPSHOW 0 / SPHIDE 0 before SPSET -> errnum 4; SPSHOW 512 / SPHIDE 512 / SPCLR 512 -> errnum 10;
  SPPAGE 6 / SPPAGE -1 -> errnum 10. All matched the disassembled predictions; folded into the 5 specs.
- [x] S-T8a CONTRADICTION RESOLVED · the corpus 1-argument no-OUT form `SPSET 510` (534NX3L6/TXT/DANMAKU3 line 380)
  is oracle-confirmed to raise errnum 4 (2026-06-22 s_t8a) — dead/buggy code behind the rare MPCOUNT!=2 branch.
  The disassembly's argcount-2..6 guard is correct; spec kept at errnum 4.
- [ ] S-T8a sprite VISUAL/state side-effects (need framebuffer oracle O-T6): SPSET creation (template vs direct
  image, attribute bits applied), SPSHOW/SPHIDE display-flag toggle, SPCLR slot-free + bulk clear-all (0-arg),
  SPPAGE render-page redirect, SPSET OUT -1 pool-exhaustion result, SPSET reinit (SPVAR -> 0). All disassembled
  + documented; runtime visual confirmation queued.
- [x] S-T8b error + round-trip values HARVESTED (2026-06-22 s_t8b): mgmt out-of-range (512) -> errnum 10 for
  SPOFS/SPROT/SPSCALE/SPHOME/SPCHR; used-before-SPSET -> errnum 4 (all five); bad argcount -> errnum 4
  (SPOFS 0,0 / SPROT 0); SPCHR defn 4096 -> errnum 10. Round-trips: SPOFS 0,50,80 OUT->50,80; SPROT 0,45->45;
  SPSCALE 0,2,0.5 OUT->2,0.5; SPHOME 0,16,16 OUT->16,16; SPCHR 0,64,64,16,16,1 OUT U,V->64,64 and full->64,64,32,32,1.
  KEY FINDING: SPROT does NOT wrap/normalize — SPROT 0,-25->-25, SPROT 0,450->450, SPROT 0,11.2->11 (truncated, verbatim).
  Spec semantics corrected; raised those sources to hw_verified.
- [ ] S-T8b remaining oracle-pending (need framebuffer oracle O-T6 or extra cases): SPOFS Z-depth round-trip
  (3-var OUT X,Y,Z value), SPCHR U+W/V+H>512 errnum (assumed 10, matching SPSET), SPCHR form-1 template round-trip
  (SPCHR mgmt,defn then OUT U,V/DEFNO — needs an SPDEF setup), and the actual on-screen transform (visible render).
- [ ] S-T8c remaining oracle-pending (needs framebuffer/composite oracle O-T6, not a VALUE harvest): the actual
  SPANIM on-screen animation output — keyframe interpolation values, negative-time linear-interp curve, the
  per-frame timing ("starts on the frame following SPANIM"), relative ("+"/+8) base accumulation, and Loop 0
  endless behavior. Error conditions + SPLINK function returns are now hw_verified (s_t8c batch 2026-06-22);
  the visible render and interpolation math are not.

- [ ] S-T8d collision read-back / coordinate VALUES (deterministic VALUE harvest, not framebuffer):
  - SPCOL OUT getters: after `SPCOL m,sx,sy,w,h,scale,mask`, read back `SPCOL m OUT ...` and confirm the
    stored scale flag (does TRUE store 1? does numeric coerce?), the mask, and the range (esp. the default
    range = sprite W,H when not explicitly set). Forms 4-7 and the leading-comma skip `SPCOL m OUT ,mask`.
  - SPHITINFO 3/5/9-var collision coordinates + velocities after a REAL swept collision (TM in 0..1, and
    X1/Y1 = pos + vel*TM). Need a deterministic moving-sprite setup harvested via the oracle.
  - SPHITINFO undocumented 3-var form `OUT TM,X1,Y1` (seen only in disassembly @0x1440f8) — confirm it is
    accepted and returns TM + object-1 coords (no corpus example found).
  - SPHITRC mask AND-filtering + swept-movement outcomes; SPHITSP swept-with-SPCOLVEC outcomes (do the
    movement vectors change a same-frame hit/miss vs the static AABB?).

- [ ] S-T8e vars/funcs/state — remaining VALUE/render harvests (core forms + error cases already
  hw_verified s_t8e batch 2026-06-22: SPVAR read/write round-trip, SPCHK stopped=0, SPUSED TRUE/FALSE,
  SPDEF defaults W=H=16/A=1 + range errnum 10, SPCOLOR &H11223344 round-trip, SPFUNC bind NOERR before
  SPSET, all mgmt-oob errnum 10 / before-SPSET errnum 4):
  - SPCHK mid-animation #CHK* bit values — need a running SPANIM to set channel bits, then read SPCHK.
  - SPDEF non-default template field read-back (U,V,W,H,OX,OY,A round-trip for explicit values; copy form 6
    field inheritance; bulk array/DATA forms; array element-count-not-multiple-of-7 -> errnum 31).
  - SPFUNC CALL SPRITE dispatch: confirm the documented "error before SPSET" actually surfaces at CALL time
    (binding itself does NOT raise); CALLIDX value inside the callback; errnum 4/8 for unresolvable / non-string label.
  - SPCLIP visual clipping rectangle effect (needs framebuffer oracle O-T6); confirm coordinate clamp vs error
    for out-of-range (X 0-399 / Y 0-239) and the start/end normalization (sx>ex swap).
  - SPVAR variable-number out-of-range (n>7) behavior — no explicit guard seen in the handler.

- [ ] S-T9a BG setup — render/side-effect harvests (error cases already hw_verified, s_t9a batch
  2026-06-22: BGSCREEN layer/area/bad-tile -> errnum 10/10/4, BGPAGE/BGCLR/BGSHOW/BGHIDE layer-oob ->
  errnum 10, BGSHOW/BGHIDE no-arg -> errnum 4). Need the BG framebuffer oracle (O-T6) for:
  - BGSCREEN 4th-arg tile-size effect (8/16/32 px tiles) on rendered output and on BGGET/coord math.
  - BGPAGE GET default value (expected 5/GRP5) and that SET changes which GRP layers fetch tiles from.
  - BGCLR clear effect (map filled with empty tiles) — one layer vs all-layers form.
  - BGSHOW/BGHIDE visibility toggle on rendered output (and idempotence).

- [ ] S-T9b BG tiles — render/side-effect harvests (error cases already hw_verified, s_t9b batch
  2026-06-22: BGPUT layer/X/Y-oob -> errnum 10; BGFILL layer-oob -> errnum 10; BGGET layer-oob ->
  errnum 10 / used-as-statement -> errnum 4; BGCOPY layer-oob -> errnum 10 / 5-arg -> errnum 4;
  BGCLIP layer-oob -> errnum 10 / 3-arg -> errnum 4; all valid forms NOERR, errline 1). Need the
  BG framebuffer oracle (O-T6) for:
  - BGPUT/BGFILL screen-data exact bit layout: the docs say rotation is at b12-b13, but the named
    constants #BGROT90=&H0800 / #BGROT180=&H1000 / #BGROT270=&H2000 (constants.yaml) place rotation
    at b11-b13. Confirm via BGPUT a value then BGGET it back which bits the engine keeps/decodes.
  - BGGET round-trip after BGPUT (read back the exact packed screen-data value); char-number cycle-1024
    behavior (does BGGET return the stored 0-4095 or the mod-1024 displayed value?).
  - BGGET pixel-mode (coordFlag=1) pixel->char conversion: rounding and which tile size is used.
  - BGFILL/BGCOPY rectangle semantics: inclusive corners, reversed start/end ordering, out-of-bounds
    coordinate clamp vs error (no errnum seen in the handler), and BGCOPY overlapping src/dst.
  - BGCLIP clip rectangle visible effect and the internal layer-id (layer+2) mapping.

- [ ] S-T9d BG animation/state — render/side-effect harvests (error + default-read cases already
  hw_verified, s_t9d batch 2026-06-22: BGANIM 2-arg -> errnum 4 / layer-oob & neg -> errnum 10;
  BGSTART/BGSTOP/BGFUNC layer-oob -> errnum 10; BGVAR layer-oob & varnum-oob(8) -> errnum 10;
  BGVAR(0,0) -> 0; BGCHK(0)/BGCHK(3) -> 0; BGCHK layer-oob & neg -> errnum 10; all errline 1).
  Still oracle-pending (need a way to run setup statements before a value read, and the BG
  framebuffer oracle O-T6):
  - BGVAR write-then-read round-trip persistence: BGVAR 0,3,7 then BGVAR(0,3) -> expect 7; var 7
    special-case (clears flag bit 0x20) observable effect; BGVAR ... OUT V form value.
  - BGCHK mid-animation #CHK* bit values while a BGANIM channel runs (which bit per XY/Z/R/S/C/V),
    and confirm BG omits #CHKUV(4)/#CHKI(8); confirm BGSTOP-then-BGCHK reads 0 on a running anim.
  - BGANIM interpolation output (positive hold vs negative linear interp), Loop 0 endless, the
    "@label" DATA form and relative "+" semantics against rendered layer transform.
  - BGFUNC callback dispatch via CALL BG (CALLIDX = layer number); errnum 4/8 for unresolvable /
    non-string labels (handler shows errnum 8 for a numeric label, errnum 4 for unresolved @Label).

- [ ] S-T9e BG load/save/color — render/round-trip harvests (error cases already hw_verified,
  s_t9e batch 2026-06-22: BGLOAD/BGSAVE/BGCOLOR layer-oob & neg -> errnum 10; BGLOAD 0,0,A 3-arg
  non-array -> errnum 8; BGSAVE 0,0,A 3-arg -> errnum 4; all errline 1). Still oracle-pending
  (needs BG framebuffer oracle O-T6 and multi-statement setup-before-read):
  - BGSAVE -> BGLOAD round trip: BGPUT a tile, BGSAVE to an array, read array contents, BGLOAD it
    back into another region and confirm the tilemap matches (cell packing: tile/palette/flip bits).
  - BGSAVE auto-grow: pass a too-small 1-D array, confirm it is resized to width*height elements.
  - BGLOAD 3-arg / 7-arg trailing numeric argument: what does it mean (start offset/index into the
    source array?) and its valid range (handler range-checks against r6=[0x165e3c], r7=r6>>20).
  - BGCOLOR set-then-get round trip: BGCOLOR 0,RGB(255,0,0) then C=BGCOLOR(0) -> expect the stored
    32-bit code (and whether the ignored alpha byte is masked off or returned verbatim).

- [ ] S-T10a BGM playback — audio output has NO deterministic emulator golden (O-T7: SB can't
  render audio to disk; emulator audio is real-time/timing-dependent). Specs pin call shape +
  arg ranges + errnum from disasm (confidence: disassembled). Deferred to O-T7 / real-hardware
  observation only (NOT a `batch` value harvest):
  - BGMCHK return value while a track is actually playing (is it always 1, or a richer flag?);
    confirm FALSE=0 / TRUE!=0 boolean on real SB.
  - BGMVAR read value while a tune with $0-$7 writes is mid-playback (handler reads live MML
    register state); confirm stopped-read == -1 and a written value round-trips during playback.
  - BGMSTOP fade-time semantics (does `BGMSTOP track,sec` audibly ramp down over `sec`?) and the
    -1 force-stop overwriting user tune 255.
  - BGMPLAY 2-frame post-trigger delay + the MML->note-event realization (MML grammar = S-C5).

- [ ] S-T10b BGM setup (BGMSET · BGMSETD · BGMCLEAR · BEEP) — specs pinned from disasm
  (confidence: disassembled); audio output has no deterministic golden (O-T7). The errnum
  cases below ARE deterministic and oracle-confirmable via `batch ... |err` (ERRNUM/ERRLINE)
  even though the audio is not — confirm and raise to hw_verified when the oracle is up:
  - BGMSET/BGMSETD: tune number out of 128..255 -> errnum 10; non-string 2nd arg -> errnum 8;
    wrong arg count (1 or 3) -> errnum 4; malformed MML -> errnum 47 (Illegal MML).
  - BGMCLEAR: tune out of 128..255 -> errnum 10; >=2 args -> errnum 4 (0-arg clears all, no error).
  - BEEP: sound number in the 134..223 gap or >383 -> errnum 10; freq outside -32768..32767,
    volume outside 0..127, pan outside 0..127 -> errnum 10; >4 args -> errnum 4. ALSO verify the
    disasm+corpus extended sound banks 224..255 and 256..383 play (no error) on real SB — docs
    only mention 0..133 but corpus uses BEEP 224/293/303/354/382 (no oracle = legal-syntax only).

- [~] S-T10d Voice & wave (TALK · TALKCHK · TALKSTOP · WAVSET · WAVSETA) — specs pinned from
  disasm (confidence: disassembled); audio/TTS output has no deterministic golden (O-T7).
  CONFIRMED on real SB 3.6.0 (Azahar) and folded in as hw_verified:
  - TALK `X=TALK(...)` (result context) -> errnum 4. (parser DOES reach the handler.)
  - TALKCHK idle `TALKCHK()` == 0; `X=TALKCHK(0)` (arg) -> errnum 4; bare `TALKCHK()` statement
    -> errnum 3 (Syntax error — function-as-statement rejected at PARSE, not the handler gate).
  - TALKSTOP `TALKSTOP 1` (arg) -> errnum 4.
  - WAVSET: 5 args -> errnum 4; defnum 223/256 -> errnum 10; attack 128 -> errnum 10;
    refpitch 128 -> errnum 10; non-string waveform -> errnum 8.
  - WAVSETA: 5 args -> errnum 4; defnum 223/256 -> errnum 10; attack 128 -> errnum 10;
    non-array source -> errnum 8. (Used scalar 6th operand: defnum/envelope checks precede the
    array-type check.)
  STILL PENDING (need a live array operand, or are audio-only — left disassembled):
  - WAVSETA refpitch/start/end-subscript out-of-range -> errnum 10, and end < start -> errnum 4:
    the array-type check precedes these, so they need `DIM A(N):WAVSETA ...,A,...` — the batch
    `|err` harness mangles colon multi-statement lines (returns spurious errnum 3); harvest with
    a single-statement form or a pre-declared persistent array.
  - WAVSET malformed-hex vs wrong-sample-count (16/32/64/128/256/512) exact errnum (disasm: 4);
    `[`/`]` repeat-marker semantics in the hex string.
  - WAVSETA 16384-sample cap + power-of-two sample-count rounding (observation only, audio).
  - TALK <S>/<P> speed/pitch realization; TALKCHK non-zero playing value mid-TALK (audio, O-T7).
- S-T11a (BUTTON/BREPEAT/STICK/STICKEX) — input-state + wireless paths need hardware:
  - Live button bitmask values per feature ID (held/pressed-repeat/pressed-no-repeat/released)
    and BREPEAT's timing effect on BUTTON feature 1: require injected button input (headless
    oracle has none). Error guards + no-input baseline (0) already hw_verified.
  - Live STICK/STICKEX axis magnitudes (Doubles clamped -1.0..1.0, ~+/-0.86 physical; Y up=+),
    and STICKEX's XON EXPAD gating: require Circle Pad / Circle Pad Pro analog input.
  - Wireless terminal-ID range check (errnum 10 vs connected-terminal count) and the
    undocumented errnum 52 (comms-not-active) path for BUTTON/STICK/STICKEX: need an active
    wireless multiplayer session (assumption: errnum 10 out-of-range, errnum 52 when inactive,
    from disasm `mov r0,#0xa`/`mov r0,#0x34`).
- S-T11b (TOUCH/ACCEL/GYROA/GYROV/GYROSYNC) — sensor/touch values need hardware:
  - Live TOUCH coordinates (TX 5..314, TY 5..234) and the no-touch STTM=0 baseline: the
    headless oracle taps the touch screen to launch RUN, so its STTM read back as 1 (contaminated
    by the launch tap), not the documented 0. Needs touch input that is NOT the harness's own tap.
    Error guards (exactly-3-OUT -> errnum 4) already hw_verified.
  - TOUCH wireless terminal-ID range (errnum 10) and the undocumented errnum 52 (comms-not-active)
    path: need an active wireless multiplayer session (assumption from disasm mov r0,#0xa / #0x34).
  - Live ACCEL axes (G), GYROA angle (rad) and GYROV angular velocity (rad/s): require enabling
    the motion sensor with XON MOTION (a feature-confirmation dialog that may hang the oracle —
    not driven live per the sb-oracle skill) plus actual device tilt. The disassembled algorithm
    (X,Y negated for ACCEL; *2π = 0x40C90FDB for GYROA/GYROV) is pinned; the no-XON errnum 37 and
    too-few-OUT errnum 4 guards are already hw_verified.
  - GYROSYNC recalibration side-effect (and the >1-call-per-frame prohibition): needs motion
    hardware; no observable return. no-XON errnum 37 and the arg-rejection errnum 4 hw_verified.
  - S-T11c MIC (MICSTART/MICSTOP/MICDATA/MICSAVE): live captured audio is not headless-
    harvestable (Azahar has no mic-input injection). UNHARVESTED: MICSTART rate/bits/seconds
    range errors (errnum 10) and the per-rate max-seconds caps; MICDATA fixed-mode position
    range (errnum 10) + loop-mode wrap + 8-bit/16-bit sample values (128-/32768-basis); MICSAVE
    recorded-range error (errnum 10) + 1D array auto-extend + the actual copy; MICSTOP stopping
    a live sampler (status 2). The shared wireless errnum 52 (comms active) needs an active
    multiplayer session. All the no-XON-MIC (errnum 36), arg-count (errnum 4) and array-type
    (errnum 8) guards are already hw_verified (s_t11c). The recording algorithm is pinned from
    the disassembly (buffer 0x01B20000, ~261760-byte cap, state struct 0x315C18).
  - S-T11d Screen control (ACLS/BACKCOLOR/DISPLAY/VISIBLE/XSCREEN): the arg-count guards
    (errnum 4) and range checks (XSCREEN mode/sprites/BG and DISPLAY-1-without-touch errnum 10)
    are hw_verified (s_t11d). UNHARVESTED — all screen-state, no scalar oracle golden:
    ACLS no-arg full reset vs the undocumented 3-arg selective reset (per-flag bitmask meaning
    of `ACLS f1,f2,f3` — bits 0x2/0x4/0x8 into worker 0x14f10c — is unknown); BACKCOLOR set/get
    color-code round-trip (the exact RGB()/alpha encoding `BACKCOLOR()` returns); DISPLAY/VISIBLE
    actual targeted-screen and layer-visibility effects. DIRECT-MODE-ONLY (RUN harness can't reach
    these, run in program mode): DISPLAY in DIRECT mode -> errnum 43, and XSCREEN 4 in DIRECT mode
    -> errnum 43 — both pinned from the disassembly but need a DIRECT-mode oracle path.

- S-T11e FADE/FADECHK (no scalar golden — screen/animation state):
    FADE's actual on-screen fader compositing (whole-screen fill in the color's alpha, drawn in
    front), the exact ARGB code `FADE()` returns for a given set color, and FADECHK() reading TRUE
    *during* a live timed fade (would need frame-timed sampling mid-animation). Error guards
    (negative time -> errnum 10; arg/result-count -> errnum 4) and the idle FADECHK()==0 ARE
    hw_verified (batch 2026-06-22, s_t11e).

- S-T12a File I/O (LOAD/SAVE/FILES/DELETE): the error guards ARE hw_verified (batch 2026-06-22,
    s_t12a): LOAD no-args -> errnum 4, LOAD/SAVE/DELETE/FILES non-string or wrong-type operand ->
    errnum 8, SAVE/DELETE read-for-value or SAVE no-args -> errnum 3. UNHARVESTED (all filesystem/
    dialog state, no scalar oracle golden — and writing files on the real SD card mutates state):
    LOAD success into program slot / GRP page / font page; the undocumented GRPn offset form
    `LOAD "GRPn:name",x,y[,dialog]` (corpus-verified syntax, runtime effect unknown); LOAD-failed
    errnum 46 (missing file) and Illegal-file-format errnum 35 (both hypothesis from docs, not
    body-pinned); TXT round-trip (SAVE "TXT:" then LOAD("TXT:") returns the same UTF-8 string);
    DAT array round-trip; FILES console listing + the auto-extended string-array output contents;
    DELETE actually removing a file. Also unresolved: `A=FILES` returns NOERR via the runtime
    harness (parser rejects it before the result-count -> errnum 3 guard) — needs a compile-error
    capture path to confirm the parse-time error class.

- S-T12b File management (CHKFILE/RENAME/USE/EXEC): the error guards ARE hw_verified (batch
    2026-06-22, s_t12b): CHKFILE non-string operand -> errnum 8, CHKFILE used as a statement ->
    errnum 4; RENAME non-string first operand -> errnum 8, RENAME with 1 arg -> errnum 3.
    UNHARVESTED (all filesystem/slot/run state, no scalar oracle golden — and they mutate the SD
    card / running program): CHKFILE existence result (TRUE/FALSE) for TXT/DAT and the undocumented
    PRG/GRP resource prefixes; RENAME actually renaming a file + the undocumented cross-resource
    retype `TXT:`->`PRG:`; USE marking a slot executable (numeric form + undocumented
    `USE "PRGn:Filename"` string form) and its out-of-range-slot errnum; EXEC loading+running a
    program (form 1 string) / executing an existing slot (form 2 numeric), the DIRECT-mode error
    (hypothesis errnum 43) and load-failed (hypothesis errnum 46), and the no-return control
    transfer. USE/EXEC are parser special forms (keyword ids 332/331) with no body-readable
    handler, so their slot validation + errnums stay hypothesis until harvested.

- S-T12c Source read (PRGGET$/PRGNAME$/PRGSIZE): the error guards ARE hw_verified (batch
    2026-06-22, s_t12c): PRGGET$ with no active PRGEDIT -> errnum 38 (the no-PRGEDIT check
    precedes the arg-count check, so PRGGET$(0) without PRGEDIT is also 38); PRGGET$(0) WITH an
    active edit target (PRGEDIT 1) -> errnum 4; PRGNAME$(4)/PRGNAME$(-1) -> errnum 10,
    PRGNAME$(0,1) -> errnum 4; PRGSIZE(4)/PRGSIZE(-1)/PRGSIZE(0,3) -> errnum 10, PRGSIZE(0,0,0)
    -> errnum 4. UNHARVESTED (all program-slot/edit state — no portable scalar golden, and they
    depend on loaded slot contents): PRGGET$ returned line text + the empty-string-past-EOF
    result + the trailing-LF strip; PRGNAME$ returned file-name strings per slot (incl. the
    empty string for a never-LOAD/SAVEd slot) and the no-arg running/last-run-slot value;
    PRGSIZE returned counts for type 0 (lines) / 1 (characters) / 2 (free characters) and the
    no-arg last-run-slot count. These need a known program loaded into a slot to produce a
    stable expected value (M6-T4 source-edit harness).

- S-T12d Source edit (PRGEDIT/PRGSET/PRGINS/PRGDEL): the arg/range guards ARE hw_verified
    (batch 2026-06-23, s_t12d): PRGEDIT 4 / PRGEDIT -1 -> errnum 10 (slot out of range; -1 is
    out of range as a SLOT though valid as the 'last line' value of the 2nd arg); PRGEDIT 0,0,0
    -> errnum 4; PRGEDIT 1:PRGDEL 0 -> errnum 10 (count 0); PRGEDIT 1:PRGSET "A","B" /
    PRGEDIT 1:PRGINS "A",1,2 / PRGEDIT 1:PRGDEL 1,2 -> errnum 4 (too many args).
    DISCOVERY (session-persistent edit target): the no-PRGEDIT errnum-38 guard is shared across
    the whole PRG* family via ONE global (0x306C14). It fires only from a COLD edit state — once
    ANY PRGEDIT succeeds in the SB session the global stays non-null, so in a warm session
    PRGGET$()/PRGSET/PRGINS/PRGDEL with no preceding PRGEDIT all returned NOERR (this run),
    refining the T12c PRGGET$ errnum-38 result (which was harvested cold). UNHARVESTED:
    (1) COLD-state errnum 38 for PRGSET/PRGINS/PRGDEL (needs a fresh SB session with no prior
    PRGEDIT — restart Azahar/SB, run the no-PRGEDIT case FIRST). (2) The stateful results with no
    scalar golden: PRGEDIT running-slot guard (errnum 4) + line-range (errnum 10) + -1=last-line;
    PRGSET line-substitution + append-on-EOF; PRGINS inserted line + flag(before/after) +
    CHR$(10) multi-line split; PRGDEL deletion + negative=delete-all. All need a known program
    loaded into a slot (M6-T4 source-edit harness).

## S-T12e — DIRECT-mode (RUN · CONT · NEW · CLEAR · LIST · BACKTRACE · PROJECT)
- Harvested 2026-06-23 (hw_verified, frozen into spec/instructions/*.yaml):
  - RUN/NEW/CLEAR/LIST/CONT/BACKTRACE/PROJECT are all usable as VARIABLE names in program mode
    (`NAME=k` then `NAME` -> k). Command-form in a program: RUN/NEW/CLEAR/LIST/CONT -> errnum 3
    (Syntax error, NOT 44 — they're plain identifiers in program mode). BACKTRACE is a real
    builtin: bare `BACKTRACE` -> NOERR (runs); `BACKTRACE 1`/`BACKTRACE 1,2` -> errnum 4.
    PROJECT set form `PROJECT "X"` -> errnum 44; `PROJECT OUT PJ$` -> NOERR (allowed in program).
- UNHARVESTED (console-render-only / DIRECT-mode-only — no scalar golden, needs O-T6 render
  capture or a DIRECT-mode driver):
  - BACKTRACE actual multi-line slot:line output after a STOP/error halt (needs a suspended
    program + console capture).
  - RUN slot-launch behavior / NEW slot-erase / CLEAR memory-init / LIST EDIT-mode switch +
    `slot:line`/`ERR` target parsing — all DIRECT-mode-only effects, not capturable as a value.
  - PROJECT OUT PJ$ returned "" on this install (default project); the real current-project
    name and the set-form project-name validation (errnum 4 length>15 / bad chars; errnum 8
    char-class) are not separable as scalar goldens in a warm program-mode session.

- S-T14a · errnum table — most entries cross-checked vs the binary errnum→string pointer
  table @0x3054f8 (errnum 0..55). Oracle (S-T14a) confirmed errnum 4 (X=ABS()), 7 (A=1/0),
  8 (S$=5), 10 (X=SQR(-1)). REMAINING to harvest a clean trigger for (table value is the
  assumption): 31 Subscript out of range — `DIM ZZ(3):X=ZZ(9)` surprisingly returned
  errnum 3 (Syntax error) via the batch wrapper, so find a standalone trigger; and the
  binary-only 48..55 (Uninitialized variable used / Protected resource / Protected file /
  DLC not found / Incompatible statement / END without call / Array is too large / Too many
  arguments) whose `desc` text is inferred, not from docs.

- S-T14b · system variables — name set verified vs binary keyword/name pool (each name's
  UTF-16LE addr recorded in spec/reference/sysvars.yaml). Oracle (S-T14b) froze TRUE=1,
  FALSE=0, VERSION=&H03060000, CALLIDX=0 (goldens) and captured HARDWARE=1/TABSTEP=4/
  SYSBEEP=1 (environment- or session-dependent, not universal). REMAINING (dynamic, no scalar
  golden in a warm program-mode session — assumption is the docs description):
  - CSRX/CSRY/CSRZ cursor position (depends on prior PRINT/LOCATE state).
  - MAINCNT frame counter (monotonic) + whether `MAINCNT=0` reset assignment is legal
    (docs list read-only, but real SB is rumored to allow reset — corpus shows only `==`).
  - FREEMEM (memory-dependent), MICPOS/MICSIZE (no mic), MPCOUNT/MPHOST/MPLOCAL (no session),
    ERRNUM/ERRLINE/ERRPRG (require a prior error), PRGSLOT/RESULT, TIME$/DATE$ (clock).
  - Confirm the read-only sysvars actually raise on assignment (which errnum) vs silently
    no-op — needs a `|err` probe per name.

- S-C1 · execution-model concept spec — model authored from docs + osb (structural) +
  documented frame layout. NOT yet read from `cia_3.6.0.lst`/oracle (all hypothesis):
  - Identifier class: confirm full-width/kana + leading-digit rule (docs say
    "alphanumeric + underscore"; SB is Japanese, osb's ASCII-only is a limitation we reject).
  - Suffix-less numeric variable: confirm dynamic Integer/Double promotion rule on mixed
    reassignment (e.g. `A=1 : A=A/2` → Integer or Double?).
  - `^` (power) operator: precedence rank + associativity (absent from osb getOPRank).
  - Exact 3.6.0 call-frame cell order (currentFunction, old bp, return addr), args-vs-locals
    overlap, and `RETURN` OUT-copy offsets — read handler from disassembly, diff vs oracle.
  - Operand-stack size / recursion depth that trips Stack overflow (errnum 5).

- S-C3 · sprite-bg-model concept spec — model authored from docs + the disassembled
  sprite/BG instruction specs (which carry handler-body reads) + hw_verified constant bits.
  Open items the model marks oracle-pending:
  - Exact per-channel mid-animation bit values reported by SPCHK (`(flags>>17)&0xFF`) and
    BGCHK (low byte) while a given SPANIM/BGANIM channel runs (see also lines for S-T8/S-T9
    above — same probe). The `#CHK*` target→bit mapping is documented; the live running bit is not.
  - Sprite SPVAR out-of-range variable number (outside 0–7): does it error (which errnum) or
    wrap/no-op? No visible guard at the SPVAR store site (BGVAR DOES guard 0–7).
  - SPHITINFO 3-variable OUT form (TM,X1,Y1) — accepted by the handler, undocumented; confirm legal + values.
  - Compositing exactness for M3 goldens (O-T6): sprite/BG draw+pivot order (scale vs rotate
    vs scroll origin), rounding, and Z tie-breaking across sprites/BG/GRP/console layers.

- S-C4 · frame-and-timing-model concept spec — model authored from docs + the disassembled
  VSYNC/WAIT specs + MAINCNT getter read (`*[0x315ec0]`). Open items the model marks
  oracle-pending:
  - MAINCNT boot value / monotonicity across RUN/NEW/CLEAR and a halt+CONT — confirm it
    never resets and never pauses (docs say "since launched"; i32 wrap point inferred).
  - VSYNC resync after a long body (body overran the target): does VSYNC return immediately
    and jump `lastVsync` to current (catch-up, dropping missed frames) or clamp? The
    `add lastVsync,count` then `str current` path suggests catch-up — pin exact semantics.
  - MAINCNT vs displayed-frame alignment under DISPLAY/XSCREEN changes and during FADE
    (assumed: counts every VBlank regardless of what is shown).
  - Confirm there is genuinely no sub-frame / high-resolution timer (none found in disasm).

- S-C5 · mml-grammar concept spec — model authored from docs (SB3 reference + manual,
  cross-checked vs SB4 mml-guide) + the disassembled BGMPLAY handler (@0x1a2d54: errnum 4 on
  argcount!=1..3; MML validate bl 0x1d44d8->0x1d475c, fail -> errnum 47; preset BGM 0-42, user
  128-255) + corpus sweep. Open items the model marks oracle-pending:
  - Tick base (192/whole-note assumed from L1-L192 divisor set) and the exact tempo T(1-512)
    -> frames(60fps) conversion — read the synth scheduler disassembly (parser core 0x1d475c).
  - `@V` velocity: confirm SB3 range 0-127 (documented in SB4, in 196 corpus programs) and how
    it scales against channel `V` (multiplicative %?).
  - SFX instrument bank ceiling: corpus uses @256..@287+ (e.g. @267, @275, @281, @287) beyond the
    single documented @256; and whether @130-@134 extra drum kits exist in SB3 (SB4-only?).
  - `!` octave-invert effect, `(`/`)` volume step size, and `Q0-8` gate's exact tick formula.
  - Channel-0 redefinition / channel-order error rules (currently cross-system from SB4).
  - `/comments/` and `|chords|`: SB3 appears to reject them (errnum 47) — confirm vs oracle.
  - `$n` assignment range (docs 0-255) and overflow/clamp behavior.

- S-C6 · file-and-extdata-format concept spec — model authored from docs (manual
  managing-projects-files + save/load/files/chkfile/project/gsave/gload) + the disassembled
  SAVE handler (@0x18e7d4: argcount guard -> errnum 3; shared resource-name parser @0x1d6d6c
  with type code <=0xe; unknown resource -> errnum 4 @0x18e898; page-range guard -> errnum 10
  @0x18e8f8; resource-type switch @0x18e930 cases 0..6) + the hw_verified extdata container
  (sb_extdata.py round-trip O-T3/O-T4) and PCBN GRP layout (sb_grp.py pixel-exact O-T6) +
  PETC corpus container (extract_sbsave.py, community 915/915). Open items oracle-pending:
  - DAT numeric-array PCBN tagging: how int vs double vs ushort element type and array
    dimensions are encoded in the PCBN header for SAVE"DAT:"/LOAD"DAT:" (only GRP image
    layout is pixel-verified).
  - GRPF font page: confirm same 512x512 PCBN layout as GRP0-5 (assumed) vs distinct size.
  - Header date field @0x0C: what real SB stamps on save (injector uses fixed DF 07 0A 0F);
    whether SB validates it on load.
  - errnum 35 (illegal format) vs 46 (load failed): which corruption modes raise which on
    real hardware (bad footer, wrong PCBN magic, truncated body).

- S-C7 · error-model concept spec — model authored from docs (error-table, stop/cont/end/break,
  system-variables) + the disassembled errnum->string formatter (FUN_001e94a8 @0x1e94a8: range
  guard (errnum-1)<=55, table base @0x3054f8 -> pool @0x2e965c..0x2e9ac0, "Internal Error"
  fallback @0x1e9588, "(detail)" append, store errnum -> *[0x315d6c]) + spec/reference/errors.yaml
  + sysvars.yaml (ERRNUM/ERRLINE/ERRPRG read-only) + hw_verified persistence (O-T5/S-T14a).
  CONT/RUN resume/launch handlers are index-dispatched DIRECT-mode keywords, not body-pinned
  (hypothesis). Open items oracle-pending:
  - Exact "resumable error" set: which errnums leave a CONT-able state vs force errnum 33
    "Can't continue" (docs only say "depending on the error type").
  - ERRPRG after a cross-slot halt: confirm = slot the halting line lives in, not the RUN slot.
  - ERRNUM clear points: exactly which ops zero ERRNUM (ACLS, CLEAR, RUN, NEW, clean END?).
  - The formatted "(detail)" text per errnum (display-only, not a value golden).
  - STOP/START suspend display: confirm literal "SLOT:line" format and whether it matches the
    error-halt display.
  - errnum 1 vs out-of-range both render "Internal Error" — confirm no other visible distinction.
