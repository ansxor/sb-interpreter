# Harvest queue — behaviors needing oracle (Citra) verification

The autonomous Ralph loop **cannot** run the emulator (the Citra/Azahar oracle is offline/
manual by design — see `harness/README.md`). So when the loop implements a behavior it
can't pin down from the docs or disassembly, it records the open question here instead of
silently guessing. A maintainer later resolves these via `harness/harvest/` and freezes the
answer into a `spec/tests/<id>.yaml` overlay (`confidence: hw_verified`), then deletes the line.

Format: `- [ ] <task/id> · <question> · assumption: <what the code currently does>`

## Open

- [ ] parser (expr-as-statement errnum) · A bare value-function/expression used as a STATEMENT
  is a Syntax error (errnum 3) on real SB 3.6.0, NOT errnum 4 — hw_verified 2026-06-24 (M7-T2 run
  10, harness/harvest/out/exprstmt.tsv): `GCOLOR()`, `RGB(1,2,3)`, `GCLS()`, `GPAGE()`, a bare `5`,
  `1+2` all → errnum 3 errline 1. Decisively NON-uniform: `ABS(5)` as a statement → errnum 4 (a
  value-returning command reached as a statement that then validates its 0-return form), so the
  3-vs-4 split is decided by the keyword/command table BEFORE the handler runs. assumption: sb-core
  routes a paren-form `NAME(args)` statement to the builtin (GCOLOR/RGB → errnum 4) or silently
  discards it (ABS/GCLS → no error); it should instead raise errnum 3 for the genuine
  expr-as-statement forms while still allowing a user-DEF `MYDEF(a,b)` paren call as a statement
  (which sb-core already supports). This is a cross-instruction parser refinement, not a single
  slice — a value-only family (GCOLOR's own value contract is hw_verified + frozen).

- [ ] M3-T4 (BGGET pixel-coord read) · The pixel→char conversion rounding (flag=1) and the
  off-map read behavior are not framebuffer-harvested. assumption: char coord = floor(pixel /
  tileSize) via `div_euclid`, then the cell index is wrapped modulo the map width/height
  (a repeating map, no error). Confirm the rounding (truncate vs floor) + whether off-map
  reads wrap, clamp, or return 0.
- [ ] M3-T4 (BGFILL out-of-bounds rectangle) · The handler shows no coordinate range guard
  (only the layer check). assumption: the fill rectangle's corners are normalized (min/max)
  and CLAMPED to the map bounds, so an OOB rectangle fills only its in-bounds intersection
  (never errors, never panics). Confirm vs an errnum-10 / no-op / wrap behavior.
- [x] M3-T4 (BGOFS Z clamp) · RESOLVED 2026-06-24 (M7-T2 run 13, hw_verified). Z is NOT
  clamped — it is RANGE-CHECKED to -256..1024 inclusive and a value outside raises errnum 10
  (1025/-257/2000/-1000 -> errnum 10 errline 1; 1024/-256 stored verbatim). X,Y ARE stored
  verbatim with no wrap/clamp (1000/5000/-1000 round-trip). Fixed sb-core (added the Z guard)
  + froze the round-trip + range cases in bgofs.yaml.
- [ ] M3-T4 (BGPUT/BGFILL malformed hex string) · screenData strings parse as ≤4-digit hex.
  assumption: an unparseable string parses leniently to 0 (no error); over-long (> 0x2000
  chars) → errnum 41; wrong type → errnum 8. Confirm the malformed-hex result + the exact
  length threshold.
- [ ] M3-T4 (BG layer default visibility) · assumption: BG layers are visible by default
  (BGSHOW not required to show BGPUT content). Confirm the power-on visibility flag (needs
  the BG framebuffer oracle, O-T6).
- [ ] M3-T2 (SPANIM runtime interpolation) · The exact per-frame value of the GRAPHICAL
  animation channels (XY/Z/UV/I/R/S/C) is not framebuffer-harvested. assumption: documented
  model — a positive `time` HOLDS the keyframe item value for that many frames, a negative
  `time` LINEARLY interpolates from the segment start to the item over `|time|` frames
  (`cur = start + (end-start)*frame/|time|`); the channel starts at the sprite's value at
  SPANIM time; relative (`+8`/`"+"`) adds that captured base; loop N then stop / loop 0
  endless. Integer channels (UV/I/C) round-to-nearest on write. Deterministically tested via
  the V channel (`SPVAR(m,7)` round-trips the value exactly — `harness/corpus/cases/sprite_anim.yaml`)
  but the graphical channels' rounding + the "starts on the frame AFTER SPANIM" timing offset
  are oracle-pending.
- [ ] M3-T2 (SPANIM form-2 DATA count) · The DATA-`@label` form's first value: docs say it is
  the KEYFRAME count; disassembly builder @0x163cf0 reads it as the TOTAL item count and
  requires divisibility by the stride. assumption: code follows the docs (first value =
  keyframe count, reads count*stride items via `read_anim_data`); set_anim still caps >32 → 39.
- [ ] M3-T2 (SPANIM non-numeric data errnum) · A non-numeric keyframe data value: builder
  @0x163d98 raises errnum 8 (type mismatch) but other builders have errnum-40 sites.
  assumption: the VM's `values_to_f64` raises errnum 8.
- [ ] M3-T2 (SPVAR variable number > 7) · The handler computes slot+0x58+n*8 with no visible
  0..7 guard (any bound is inside FUN_001eec7c). assumption: code rejects n∉0..7 with errnum 10.
- [ ] M3-T2 (SPFUNC dispatch + unresolved label) · the errnum for a label/process that does not
  resolve is errnum 4 per disassembly but unconfirmed. assumption: bind records the resolved
  name; an unresolvable name raises errnum 4.
  UPDATE 2026-06-23 (M6-T6): `CALL SPRITE`/`CALL BG` dispatch IS now implemented (sweep the
  table in ascending order, invoke each SPFUNC/BGFUNC-bound process with CALLIDX = the sprite
  mgmt / BG layer number; @label → GOSUB-style, DEF → 0-arg CALL). Built to osb VM.d
  CallSprite/CallBG (the only structural reference — these are parser special forms with no
  body-readable disassembly), confidence community. STILL ORACLE-PENDING:
    - does CALL SPRITE raise at CALL time for a bound-but-NOT-SPSET sprite? (docs say "If used
      before SPSET, an error will occur" but the bind itself doesn't raise — hw_verified; we
      currently run the callback regardless of SPSET, following osb isCallable which ignores it)
    - the exact final CALLIDX value after a sweep (we leave it one past the table, e.g. 4 after
      CALL BG, per osb; whether real SB clears it to 0 or leaves it is unconfirmed)
    - does a nested CALL BG inside a sprite process clobber the shared CALLIDX counter (osb's
      one-shared-counter model says yes — the "SPFUNC 1~511 not called" quirk; we share one
      counter so we'd reproduce it, but it's unverified on real SB)
    - exact iteration upper bound for sprites (we sweep 0..511 = SPRITE_COUNT; osb uses spmax,
      per-screen, which may differ for the upper screen / XSCREEN modes)

- [ ] M2-T2 (drawing-primitive pixel coverage) · GPSET/GLINE/GBOX/GFILL/GCIRCLE/GTRI/GPAINT
  are IMPLEMENTED in sb-core (`crates/sb-render/src/raster.rs`) with deterministic integer
  rasterizers (Bresenham line/box, midpoint circle, edge-function triangle fill, stack flood
  fill) writing the RGBA5551 manip page; their call-shape / arg-count / default-color behavior
  is hw_verified and replays in the conformance gate. The EXACT pixel coverage (line endpoint
  rule, the circle/arc midpoint vs hardware, GCIRCLE arc/sector angle convention, GPAINT
  boundary test, triangle edge inclusivity, float→int coordinate rounding) is faithful-but-
  unverified — it has no scalar golden and is the same work already queued under **S-T7b** /
  **S-T7c** "visual side-effects (framebuffer path)" above. When O-T6 grows a framebuffer/PNG
  golden path (M2-T5), harvest per-primitive goldens and pixel-diff the rasterizers against
  real SB 3.6.0, correcting any algorithm that diverges.

- [ ] M2-T5 / GLINE + GTRI diagonal rasterization DIVERGES from the device — RE the handler.
  The M2-T5 golden gate is live and the committed goldens are **hw_verified oracle GRP
  captures** (gcls_blue, gpset_corners, gfill_box, gcircle_mid, scene_mixed all pixel-EXACT vs
  real SB 3.6.0). Harvesting surfaced a real bug: **GLINE diagonals don't match**. For
  `GLINE 0,0,399,239` the device plots y per x as `0,0,1,1,2,3,3,4,4,5,…` = `floor(0.6·x)`
  (slope 0.6 = 240/400, a fixed-point DDA), while sb-core's textbook Bresenham (dx=399,dy=239,
  slope 0.599) plots `0,1,1,2,2,3,4,4,5,5,…` (638/96000 px differ on the cross). **GTRI**
  diverges the same way (its diagonal edges; the original triangle scene differed 159px at the
  apex). Axis-aligned runs match (GBOX, horizontal/vertical GLINE) and GCIRCLE's midpoint
  matches exactly, so this is specifically the diagonal line/edge stepping. Fix in M2-T2:
  read the GLINE/GTRI handler in the disassembly (sb-disasm) to pin the exact slope/DDA +
  rounding, change `crates/sb-render/src/raster.rs` to match, then add `gline_cross.sb3` (+ a
  GTRI scene) back as committed goldens and re-harvest. (Supersedes the generic "line endpoint
  rule / triangle edge inclusivity" sub-items of the M2-T2 entry above for the diagonal case.)

- [ ] M1-T14 (ENDIF leading-comment quirk) · A LEADING stray `ENDIF` raises errnum 28, but
  `REM X` + newline + `ENDIF` raises NO error at all (sb-oracle 2026-06-23) — a leading comment
  line suppresses the stray-ENDIF check. sb-core does NOT model this (it raises 28 for `REM
  X\nENDIF` because REM lexes to a bare Newline, so the ENDIF is still the first statement). ·
  Find the exact rule (is it any leading newline, or specifically a comment?). Probe: `\nENDIF`
  (blank first line), `:ENDIF`, `'X\nENDIF`. Most IF-block mismatches → 3, leading ENDIF → 28
  all hw_verified 2026-06-23 (if/endif/else.yaml + structural_errnums.yaml).
- [ ] M1-T14 / S-T14c (undefined `#const`) · What does real SB 3.6.0 do with an UNDEFINED
  `#NAME` (one not among the 79 built-ins) — e.g. `PRINT #NOTACONST` / `DATA #FOO`? Syntax
  error 3 at parse, an undefined-variable error, or silently 0? · assumption: the 79 known
  `#NAME` constants now fold to their hw_verified value (`sb_core::consts`); an unknown
  `#NAME` falls through to the undeclared-variable path → 0 (likely wrong — probe a bare
  `#ZZZ`).

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
  Open questions: (a) RESOLVED 2026-06-23 (M7-T2): the MP-restriction flag @0x305612 IS zero
  in a normal program-mode RUN context. Proof: the post-flag validation guards fire — `MPSET
  -1,0`/`MPSET 9,0` -> errnum 10, `MPSTART 1,"X"`/`5,"X"` -> errnum 10, `MPSTART 4,99` ->
  errnum 8, `MPSTART 4,<17char>` -> errnum 10 (all sb-oracle Azahar) — NOT errnum 52. So a
  valid MPSTART proceeds toward a real connection (do NOT harvest it headless — it hangs).
  errnum 52 stays oracle-pending (no headless way to set the flag). (b) MPSTART RESULT value
  on success/failure/timeout; (c) MPSTAT 0/1 return
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
  COPY DATA-form with an UNDEFINED "@Label" (guess 14 Undefined label, by analogy with RESTORE);
  FILL with offset/count beyond array bounds (errnum 31?); SORT/RSORT with bad/missing array
  args (errnum 4?). · assumption: per docs/error-table. Core COPY (incl. 5-arg + DATA form),
  FILL (incl. string+offset), SORT/RSORT (numeric/float/string keys + parallels) all
  hw_verified 2026-06-22. IMPLEMENTED in sb-core (M1-T14 increment 2026-06-23): COPY/FILL now
  run; the unharvested error edges are coded to the above assumptions — COPY short DATA → 13,
  COPY undefined label → 14, FILL out-of-bounds offset/count → 31 (these three stay
  oracle-pending, NOT yet hw_verified).
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
- [x] M1 (general) · Exact `STR$`/PRINT double→string formatting (sig figs, exponent
  threshold, trailing zeros) · RESOLVED by M7-T4 (2026-06-23): STR$=C `%g`/6-sig (handler
  @0x1eb2a8, fmt "%g" @0x1eb4a8) — `format_g` verified against a 2000-case bit-exact sweep +
  oracle; PRINT=C `%.8f`+trailing-zero/dot trim (handler @0x180a50, fmt "%.8f" @0x180b0c),
  NOT %g — `format_print_number`/`format_fixed8`, hw_verified via console read-back. Both
  keep signed zero (STR$(-0.0)/PRINT -0.0 → "-0"). See str.yaml/print.yaml.
- [ ] M1-T1 (lexer) · Is `1E5` lexed as `1` + ident `E5` (no exponent literal)? · assumption:
  yes (osb behavior) — confirm against 3.6.0.
- [x] S-T1b (CLASSIFY) · RESOLVED 2026-06-23 (M7-T2): inf->1, NaN->2 hw_verified via overflow.
  `EXP(1000)` overflows the double to +inf and `EXP(1000)-EXP(1000)` is NaN, so
  CLASSIFY(EXP(1000))=1, CLASSIFY(-EXP(1000))=1, CLASSIFY(EXP(1000)-EXP(1000))=2 on real SB
  3.6.0 — confirming the @0x20c3e0 helper mapping. classify.yaml now hw_verified.
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
- [x] S-T1f (RND/RNDF/RANDOMIZE) · DONE 2026-06-23 (M1-T9). Harvested the seeded sequence via
  sb-oracle prog-cases: `RANDOMIZE 0,1` then RND(100) = 89,33,33,52,...66 and RNDF(0) = 0.836095
  (matches otya_test.sb3 exactly); `RANDOMIZE 5,1`->RND(5,100) = 89. Folded into
  rnd/rndf/randomize.yaml as hw_verified tests + the rng.rs TinyMT32 impl is bit-exact. RND
  reduction is plain `raw % max` (reduce helper @0x1fd4e8); RNDF is two-draw 53-bit
  (a>>5)*2^26+(b>>6))*2^-53 (core @0x1eac60); RANDOMIZE = tinymt32_init, no extra draw (@0x26f580).
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
- [x] S-T4f KEY() function form · RESOLVED hw_verified 2026-06-24 (M7-T2 run 4): `KEY n,"S":
  ?KEY(n)` reads back the bound string (KEY(3)="HI", KEY(1)="CLS", KEY(5)="HELLO"); function
  form honors 1..5 (KEY(6)/KEY(0) -> errnum 10), two args -> errnum 4. Frozen in
  spec/tests/key.yaml; key.yaml confidence -> hw_verified. sb-core already matches.
- [x] S-T4f OPTION STRICT/DEFINT/TOOL behavior · RESOLVED hw_verified 2026-06-24 (M7-T2 run 4):
  OPTION STRICT + undeclared `B=2` -> errnum 15 errline 2; OPTION TOOL compiles cleanly (no
  error, recognized feature); unknown feature -> errnum 3. OPTION DEFINT flips the suffix-less
  numeric default Real->Integer with truncation-toward-zero (A=3.7->3, A=-3.7->-3, A=2.5->2,
  A=-2.5->-2, A=5->5, A#=3.7->3.7); WITHOUT any OPTION the suffix-less default is Real
  (A=3.7->3.7, A=2.5->2.5, `DIM A[3]:A[0]=3.7`->3.7). Frozen in spec/tests/option.yaml;
  option.yaml confidence -> hw_verified. (The 4 DEFINT real->int coercion cases are NOT frozen
  yet — see the sb-core impl gap below.)
- [ ] M1-T4 sb-core: suffix-less numeric default + OPTION DEFINT (hw_verified, fix needed) ·
  Real SB 3.6.0 defaults a suffix-less numeric to **Real (Double)** and `OPTION DEFINT` flips
  that default to **Integer** (truncating toward zero). sb-core: scalar auto-vars / `VAR A` are
  correctly Real, BUT (a) an unsuffixed `DIM` array defaults to Int (`bytecode::VarType::
  from_suffix(None)->Int`), so `DIM A[3]:A[0]=3.7` -> 3 here vs 3.7 on hardware; and (b)
  `OPTION DEFINT` is parsed + recorded (`compiler.rs:273`) but never consumed -> no-op, so
  `OPTION DEFINT:A=3.7` -> 3.7 here vs 3 on hardware. Fix: default unsuffixed numeric element/
  scalar type to Real, and make DEFINT flip it to Int (then the 4 queued DEFINT coercion cases
  in spec/tests/option.yaml can be frozen). hw_verified values are recorded in option.yaml's
  sources. NOTE: this changes the default array element type for unsuffixed `DIM` arrays — sweep
  existing array tests when implementing.
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
- [ ] S-T5a LOCATE column-50 print behavior · `LOCATE 50,0` sets the cursor to the off-screen
  right edge (X=50 is accepted, 0..49 is displayable). What does `PRINT "X"` do there — wrap
  to column 0 of the next row, drop the character, or something else? sb-core currently wraps
  and the scrape becomes "\nX". · assumption: wraps like any past-right-edge cursor (osb behavior);
  needs console-grid/screenshot oracle to confirm.
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
- [x] S-T7d GLOAD/GSAVE error edges · HARVESTED 2026-06-23 (sb-oracle batch s_t7d_bitmap, M2-T3):
  GLOAD too-small image_array → errnum 31 (3-arg whole DIM W[8] and 7-arg DIM W[3], both 31);
  GLOAD/GSAVE string array → errnum 8 (Type mismatch). Folded into gload.yaml/gsave.yaml (hw_verified).
  Still oracle-pending: negative Width/Height → errnum 10 (disasm-only; implemented but not yet harvested).
- [x] S-T7d visual/array side-effects · HARVESTED 2026-06-23 (sb-oracle batch s_t7d_bitmap, M2-T3) WITHOUT
  the framebuffer oracle — the GSPOIT scalar-read path makes blits/transfers deterministically checkable:
  GCOPY moves pixels (red GPSET 10,10 → GCOPY 0,0,32,32,100,100,1 → GSPOIT 110,110 = red); copy_mode 0
  SKIPS the transparent source (destination kept), mode 1 OVERWRITES with transparent (→ 0). GSAVE element
  word format: flag 1 = raw RGBA5551 (red 0xF801=63489), flag 0 = 32-bit logical ARGB (red 0xFFF80000 =
  -524288 signed / 4294443008 in a Double array); 1-D dest auto-expands (whole-area 262144, 8×8 = 64).
  GSAVE/GLOAD round-trip a pixel exactly for both flags. Folded into graphics_bitmap.yaml + the specs
  (hw_verified). The pixel-EXACT PNG golden of a blit is still M2-T5 (compositor + O-T6).
- [ ] M2-T3 GRPF (source page -1) content · GCOPY/GSAVE with src_page -1 (GRPF, the captured-screen plane)
  is accepted (no error, hw_verified) but GRPF is not backed in the GRP model — reads yield transparent
  pixels. The real GRPF content needs the framebuffer/screen-capture model (O-T6). Implemented as blank; queued.
- [ ] M2-T3 GLOAD form-2 (palette array) semantics · implemented as the documented index→palette recolor
  (image word = palette index; palette entry read as a 32-bit logical color → device). The corpus confirms the
  syntax is real (`...,CHIP8_PAL,TRUE`); the EXACT palette interpretation (entry format, OOB-index behavior,
  copy_mode interaction) is oracle-pending (needs the framebuffer oracle or a GSPOIT round-trip harvest).
- [ ] M2-T4 compositor Z model · default per-layer Z values (GRP default GPRIO=0; the console/BG/sprite
  default Z) and the exact equal-Z tie-break order are oracle-pending — they need the *composite* framebuffer
  capture (O-T6, screenshot path), not the single-page GRP round-trip (already done). The compositor paints
  rear→front by Z (smaller draws in front) and breaks an equal-Z tie by the documented layer order
  GRP<BG<sprite<console (stable slice order); CONSOLE_DEFAULT_Z=0 is an assumption. Harvest a 2-layer overlap
  (GRP at GPRIO p vs console) at several p to pin the console's true plane + the tie-break.
- [ ] M2-T4 backdrop / BACKCOLOR composite · the compositor takes an ARGB8888 backdrop (DEFAULT_BACKDROP =
  opaque black); the exact BACKCOLOR→backdrop mapping and its default are oracle-pending (composite capture,
  O-T6). Harvest BACKCOLOR c then screenshot an otherwise-empty screen to confirm the backdrop color + default.
- [ ] M2-T4 partial-alpha composite rule · the device GRP page is 1-bit alpha (compositor uses an alpha-bit key:
  opaque covers, clear shows through). How 8-bit sprite/console alpha composites over GRP/BG is an O-T6 composite
  question (queued) — M2 has only the 1-bit key; revisit when sprites/BG land (M3) with composite goldens.
- [ ] S-T7e color read (GSPOIT · RGB · RGBREAD) · Value/error expects HARVESTED (sb-oracle 2026-06-22 s_t7e):
  GSPOIT off-page -> 0 (NOT -1 as PTC docs claim); RGB clamps channels to 0-255 (RGB(999,999,999)=-1);
  RGB/GSPOIT arg-count errors -> errnum 4. RESOLVED 2026-06-23 (s_c2): (a) GSPOIT post-draw round-trip is
  HARVESTED via the multi-statement prog path (GPSET x,y,RGB(...):GSPOIT(x,y)) — RGB(255,0,0)->-524288,
  RGB(255,255,255)->-460552 (==#WHITE), RGB(0,100,0)->-16752640; folded into GSPOIT.yaml tests (hw_verified)
  and spec/concepts/screen-and-color-model.md. RESOLVED 2026-06-24 (M7-T2 run 9): (b) RGBREAD value round-trip
  HARVESTED hw_verified via the batch `prog`/`progstr` path (setup `RGBREAD ... OUT R,G,B`, capture STR$(R)+...):
  &HFF804020->128,64,32; 4-OUT &H80FF8040->128,255,128,64; -1->255,255,255,255; 0->0,0,0,0; RGB(160,128,96)
  round-trip->160,128,96; corpus alpha-only RGBREAD &H80FF8040 OUT A,,,->A=128; errnum 2-OUT/5-OUT->4, string
  color->8. RGBREAD top-level flipped to hw_verified; harness/harvest/out/rgbread.tsv. STILL PENDING:
  (c) GSPOIT errnum 49 (0x31) graphics-state guard @0x1543bc — undocumented
  (beyond the 3-47 table), not reachable from ordinary user code; confirm trigger if ever possible.
- [x] S-T8a sprite lifecycle (SPSET · SPCLR · SPSHOW · SPHIDE · SPPAGE) · ERROR expects HARVESTED hw_verified
  (sb-oracle 2026-06-22 s_t8a): SPSET 512,0 / -1,0 -> errnum 10; SPSET 0,4096 -> errnum 10; SPSET 0,0,0,0,0,0,0
  -> errnum 4; SPSHOW 0 / SPHIDE 0 before SPSET -> errnum 4; SPSHOW 512 / SPHIDE 512 / SPCLR 512 -> errnum 10;
  SPPAGE 6 / SPPAGE -1 -> errnum 10. All matched the disassembled predictions; folded into the 5 specs.
- [x] S-T8a CONTRADICTION RESOLVED · the corpus 1-argument no-OUT form `SPSET 510` (534NX3L6/TXT/DANMAKU3 line 380)
  is oracle-confirmed to raise errnum 4 (2026-06-22 s_t8a) — dead/buggy code behind the rare MPCOUNT!=2 branch.
  The disassembly's argcount-2..6 guard is correct; spec kept at errnum 4.
- [ ] S-T8a sprite VISUAL side-effects (need framebuffer oracle O-T6): the ATTRIBUTE bits actually applied
  to the rendered sprite (rotation/flip/additive), and SPSET reinit clearing SPVAR -> 0 (the SPVAR reset is
  separately checkable via the SPVAR scalar — queue). All disassembled + documented; runtime visual
  confirmation queued.
  (SPSET creation [template + direct + auto-allocate + range + OUT -1 pool-exhaustion] RESOLVED 2026-06-24
  M7-T2 — hw_verified via SPUSED read-back + the auto-allocate return value, no framebuffer; SPSET now
  confidence: hw_verified. SPCLR slot-free + bulk clear-all (0-arg) RESOLVED earlier the same day, same way.)
- [x] M3-T1 SPSET direct-image source-rect overflow errnum RESOLVED (2026-06-24 M7-T2): `U+W`/`V+H` > 512 is
  errnum 10 (matches sb-core's `rect()` assumption). hw_verified: SPSET 0,500,0,20,16 (U+W=520) and
  SPSET 0,0,500,16,20 (V+H=520) -> errnum 10; the U+W==512 edge (SPSET 0,496,0,16,16) is accepted. Disasm:
  cmp #0x44000000 (512.0) / bgt errnum-10 @0x141b44 (U+W) + @0x141b5c (V+H).
- [x] M3-T1 SPSET auto-allocate scan tie-break RESOLVED (2026-06-24 M7-T2): the OUT/function forms pick the
  LOWEST free slot (hw_verified IX=SPSET(0)->0, with 0,1 taken ->2; range IX=SPSET(100,105,0)->100, with 100
  taken ->101, IX=SPSET(5,5,0)->5, full single-slot range ->-1). DISCOVERY: forms 5/6 require upper <= lower —
  a reversed range IX=SPSET(105,100,0) raises errnum 4 (NOT a downward scan; cmp/ble @0x141a30). Fixed
  `SpriteState::alloc` (forward-only) + added `alloc_range` errnum-4 guard in builtins/sprite.rs; SPSET now
  confidence: hw_verified.
- [x] S-T8b error + round-trip values HARVESTED (2026-06-22 s_t8b): mgmt out-of-range (512) -> errnum 10 for
  SPOFS/SPROT/SPSCALE/SPHOME/SPCHR; used-before-SPSET -> errnum 4 (all five); bad argcount -> errnum 4
  (SPOFS 0,0 / SPROT 0); SPCHR defn 4096 -> errnum 10. Round-trips: SPOFS 0,50,80 OUT->50,80; SPROT 0,45->45;
  SPSCALE 0,2,0.5 OUT->2,0.5; SPHOME 0,16,16 OUT->16,16; SPCHR 0,64,64,16,16,1 OUT U,V->64,64 and full->64,64,32,32,1.
  KEY FINDING: SPROT does NOT wrap/normalize — SPROT 0,-25->-25, SPROT 0,450->450, SPROT 0,11.2->11 (truncated, verbatim).
  Spec semantics corrected; raised those sources to hw_verified.
- [x] SPCHR full VALUE contract HARVESTED + IMPLEMENTED (2026-06-24, M7-T2 run 27): U+W/V+H>512 -> errnum 10
  (was assumed); form-1 template round-trip (copies source rect + origin->home + attr bits1-5 + defno, preserves
  position); GET DEFNO (template#/SPSET defn/0-after-direct); 3-return form = U,V,ATTRIBUTE not width; attr SET
  preserves display bit b00; W,H round-trip verbatim through rotation; skip-empty keeps current / absent defaults.
  SPCHR was UNimplemented in sb-core — now implemented (builtins/sprite.rs::spchr + sb-render chr_template/
  set_attr_keep_display/get_attr), in IN_SCOPE_SPRITES, spchr.yaml -> hw_verified. TSVs spchr_rt*.tsv.
- [ ] S-T8b remaining oracle-pending (need framebuffer oracle O-T6): SPOFS Z-depth round-trip
  (3-var OUT X,Y,Z value) and the actual on-screen transform (visible render).
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
  - M3-T3 IMPLEMENTATION NOTES (sb-core, code task — what's modeled vs pending). Implemented as
    STATIC AABB: SPHITSP/SPHITRC overlap = strict-inequality AABB of the SPCOL detection rect placed
    at the sprite's SPOFS position (+ SPLINK inheritance), AND-mask filtered; SPHITINFO time is always 0
    (= "position at detection") and coords are the SPOFS positions. PENDING: (a) the swept/time math
    (does a non-zero SPCOLVEC/move ever flip a same-frame hit, and what TM?); (b) whether touching edges
    count as a hit (we say no); (c) the scale-adjust flag — we multiply detection W,H by |SPSCALE|, the
    exact "only affects later SPSCALE" timing is unverified; (d) SPDEF non-default field read-back +
    whether form-2-vs-form-6 (define vs copy) is really disambiguated by a skipped/`,,` arg or argcount
    (we treat argc==2 OR any Void override as the copy form); (e) the real spdef.csv default-template
    rectangles (we seed every template to 16×16 at origin 0,0 attr 1).

- [ ] S-T8e vars/funcs/state — remaining VALUE/render harvests (core forms + error cases already
  hw_verified s_t8e batch 2026-06-22: SPVAR read/write round-trip, SPCHK stopped=0, SPUSED TRUE/FALSE,
  SPDEF defaults W=H=16/A=1 + range errnum 10, SPCOLOR &H11223344 round-trip, SPFUNC bind NOERR before
  SPSET, all mgmt-oob errnum 10 / before-SPSET errnum 4):
  - [x] SPCHK mid-animation #CHK* bit values — RESOLVED 2026-06-24 (spstartstop_rt.tsv): a running XY-channel
    SPANIM gives SPCHK=1 (#CHKXY), goes to 0 under SPSTOP, back to 1 under SPSTART. Frozen in spchk.yaml
    (running_xy_bit) + spstart/spstop value-contract tests; SPSTART/SPSTOP both promoted hw_verified.
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
  - BGVAR var-7 flag-bit-0x20 side effect (the BGANIM "V"-channel marker the var-7 write clears) —
    observable only through a running BGANIM transform, so O-T6-pending. [The write→read round-trip
    + OUT-V form VALUE persistence is now hw_verified — bgvar_rt 2026-06-24, frozen in bgvar.yaml.]
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
  - [x] BGCOLOR set-then-get round trip — RESOLVED 2026-06-24 (M7-T2 run 15, hw_verified,
    bgcolor_rt.tsv): the stored 32-bit code is returned VERBATIM — the alpha byte is NOT masked
    off (decisive contrast with BACKCOLOR's 24-bit strip). &H7F112233->&H7F112233, -1->&HFFFFFFFF,
    &H80FF8040->&H80FF8040, default &HFFFFFFFF. Confirms apply helper FUN_001163c8 `str r1,[+0x30]`
    (no AND mask). Frozen in bgcolor.yaml; confidence flipped to hw_verified.

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
    are hw_verified (s_t11d). HARVESTED 2026-06-23 (M7-T2 run 2): BACKCOLOR set/get round-trip
    is now hw_verified — `BACKCOLOR()` returns the stored color masked to 24 bits (`& &H00FFFFFF`,
    the alpha/high byte is dropped; no channel swap); DISPLAY() get returns the active screen id
    (0 Upper default, 1 after XSCREEN 2:DISPLAY 1). Both bumped to top-level hw_verified.
    STILL UNHARVESTED — screen-state, no scalar oracle golden: ACLS no-arg full reset vs the
    undocumented 3-arg selective reset (per-flag bitmask meaning of `ACLS f1,f2,f3` — bits
    0x2/0x4/0x8 into worker 0x14f10c — is unknown); VISIBLE actual layer-visibility effect; the
    rendered border color BACKCOLOR produces and the physical screen DISPLAY targets. DIRECT-MODE-ONLY (RUN harness can't reach
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
    retype `TXT:`->`PRG:`. (UPDATE 2026-06-23, M6-T6: the USE/EXEC slot-validation errnums are
    now HARVESTED + hw_verified — see exec.yaml/use.yaml. USE: -1/4→10, 0→4 (running slot),
    1→ok; string PRG0:X→4, PRG1:X→46, PRG4/PRG5/FOO→4, empty-name→4. EXEC: -1/4→10, empty
    slot 1→3, FOO:X→4, missing file→46. The error model + a slot-executable flag are
    implemented + tested. STILL UNHARVESTED — the multi-program control transfer itself,
    which needs ≥2 programs loaded in a multi-slot oracle harness, NOT a single injected
    program: EXEC loading+running a program (form 1 string) / executing an existing slot
    (form 2 numeric) and its no-return-vs-END-returns-across-slots rule; per-slot vs shared
    (COMMON) variable scoping; whether USE must precede EXEC; cross-slot DEF/label resolution
    from a USE'd slot; EXEC DIRECT-mode error (hypothesis errnum 43). USE/EXEC are parser
    special forms (keyword ids 332/331) with no body-readable handler, so the transfer
    semantics stay oracle-pending until a multi-slot harness exists.)
    (UPDATE 2026-06-23, M6-T6: cross-slot COMMON DEF dispatch is now IMPLEMENTED to the
    documented model — `CALL "name"` after `USE n` resolves a COMMON DEF in a loaded,
    USE'd slot and invokes it with the same arg/OUT/return-value semantics as a same-slot
    DEF; an un-USE'd slot or a non-COMMON DEF → Undefined function (16). The VM swaps the
    target slot's program/globals into the active context for the call and restores the
    caller's on return (`Vm::load_slot_program` seeds slots 1-3; 6 new vm.rs e2e tests).
    STILL UNHARVESTED for COMMON specifically: whether a global referenced inside a COMMON DEF
    binds to the DEFINING slot's globals (what we implement) or the CALLER's — needs a 2-slot
    oracle that shares a same-named global.
    (UPDATE 2026-06-23, M6-T6: the COMMON-DEF variable scoping is now GROUNDED IN osb STRUCTURE
    and LOCKED BY TESTS, though still oracle-pending for hw_verified. osb compiler.d:318-345 — a
    bare name inside a DEF resolves to a GLOBAL only when that global already exists in the slot's
    table (else a fresh DEF-local); osb VM.d:585-589 — a COMMON-function call does
    `if (func.isCommon) setCurrentSlot(func.slot.index)`, so a global read/write inside a COMMON
    DEF resolves against the DEFINING slot's globals. sb-core matches (activate_slot swaps to the
    defining slot's globals); two vm.rs e2e tests lock it: `cross_slot_common_def_global_binds_to_defining_slot`
    (a COMMON DEF reads its own slot's zero-init global, not the caller's 7) and
    `_global_write_isolated_from_caller` (a COMMON DEF's write to G hits its own slot's global,
    caller's same-named G stays 7). Still needs a ≥2-slot oracle to confirm 3.6.0 actually does
    defining-slot binding (community/structural confidence; common.yaml source added).
    SEPARATE BUG FOUND (not M6-T6, orthogonal — likely M1-T6/DEF return path): `RETURN <bare
    global>` from a DEF used as a value function (`V=GETG()` where `DEF GETG`/`RETURN G`) raises
    Stack underflow (errnum 6) even SINGLE-slot, instead of returning the global's value. osb
    zero-inits locals on entry (VM.d:578-584) and reads globals fine; sb-core's value-return of a
    bare global underflows. Worth a follow-up: confirm SB returns the value (expect the global's
    contents) and fix the RETURN-of-global compile/run path.)
    (UPDATE 2026-06-23, M6-T6: EXEC FORM 2 numeric loaded-slot CONTROL TRANSFER is now
    IMPLEMENTED to the documented model (`Vm::exec_transfer`) — `EXEC n` on a non-running slot
    holding a loaded program switches the active program/globals to that slot, runs it from the
    top, and discards the caller's frames/GOSUB/operand-stack/DATA-cursor (impossible to return
    to the previous program); an empty slot stays Syntax error (3). The EXEC'd program runs
    against its own globals; ERRPRG reports the target slot. 5 new vm.rs e2e tests. STILL
    VmError::Unsupported / UNHARVESTED (need a ≥2-slot oracle harness, NOT a single injected
    program): the running-slot RESTART, the nested END-returns-across-slots rule, and
    the per-slot vs shared variable-scoping confirmation. EXEC DIRECT-mode error (hypothesis
    errnum 43) also unharvested.)
    (UPDATE 2026-06-23, M6-T6: the EXEC/USE FORM 1 string `"PRGn:file"` file LOAD is now
    IMPLEMENTED (`Vm::compile_slot_file` — reads the TXT body from storage, parses + lowers it
    with the in-VM `StdBuiltins` pipeline, no external host hook). `EXEC "PRGn:file"` into a
    non-running slot loads + transfers control; `USE "PRGn:file"` loads + marks the slot
    executable so its COMMON DEFs resolve cross-slot via `CALL "name"`. 7 new vm.rs e2e tests.
    Two corpus/oracle-pending bits remain: (a) which errnum a *malformed* stored program raises
    on load — we map a parse/compile failure to Syntax error (3) as a hypothesis; (b) the
    bare-name (no `PRGn:`) default-slot selection and the slot-0-as-non-running-target edge stay
    VmError::Unsupported. The documented success path itself needs no oracle — it is deterministic
    over MemStorage.)

    (UPDATE 2026-06-23, M6-T6: the numeric RUNNING-slot RESTART `EXEC 0` is now IMPLEMENTED to
    the documented model (`Vm::restart_active_slot`): re-initialise the running slot's globals to
    their declared-type zeros + discard all execution state (DEF/GOSUB/operand stacks, DATA
    cursor) + jump to the top — a fresh re-run, the corpus "restart the game" idiom (HNZBUS
    `EXEC 0`). Deterministic VM e2e test uses a DAT file counter (persists across the restart) to
    terminate after 2 passes. STILL ORACLE-PENDING: whether real SB clears variables on `EXEC 0`
    (sb-core clears — the only coherent restart, otherwise the re-run's `DIM` would redim) vs
    preserves them. The single-program oracle harness CANNOT confirm this: a self-restart needs a
    persistent counter across the restart, which means mid-program SAVEs whose confirm dialogs the
    harness only clears once → the next restart's SAVE dialog hangs. Needs the deferred multi-slot
    / file-diff harness.)
    (UPDATE 2026-06-23, M6-T6: the STRING RUNNING-slot file LOAD `EXEC "PRG0:file"` is now
    IMPLEMENTED to the documented model (`Vm::load_into_running_slot`): read the TXT body from
    storage, compile in-VM, REPLACE the running program (`Vm::program`/`globals`), discard all
    execution state + jump to the top — documented form 1 applied to the running slot, the corpus
    loader idiom (a slot-0 loader EXECs the real program). Missing file → 46 unchanged. 3 e2e
    tests. STILL VmError::Unsupported / oracle-pending: the BARE-name (no `PRGn:`) default-slot
    file LOAD — its destination slot is the deferred loader, same edge as a `PRG0:` resource when
    slot 0 is NOT the running slot — plus per-slot vs shared variable scoping.
    These need a ≥2-slot oracle harness, not a single injected program.)
    (UPDATE 2026-06-23, M6-T6: the cross-slot END-returns rule is now IMPLEMENTED to the
    DOCUMENTED model (`Vm::exec_transfer` saves the launcher's resume state into `exec_returns`;
    `Vm::try_exec_return` restores it when the EXEC'd program hits END / end-of-code, resuming
    right after the EXEC; nesting is LIFO). Both EXEC forms' notes document this ("possible to
    return by using END in a program started with EXEC in another SLOT"); a SAME-slot EXEC
    (restart / `EXEC "PRG0:…"`) saves no return. osb/VM.d Exec.execute is the structural model.
    4 new vm.rs e2e tests (cross-slot return, X preserved across return, LIFO nesting, GOSUB
    state preserved across the return). STILL ORACLE-PENDING: the exact resume-state granularity
    real SB preserves across the return (sb-core preserves the full operand stack + DEF frames +
    GOSUB stack, matching osb; whether 3.6.0 preserves all of these or only the pc needs a
    ≥2-slot oracle to confirm), and whether a free variable inside the returned-to launcher sees
    any state the EXEC'd program mutated in a COMMON.)
    (UPDATE 2026-06-23, M6-T6: the BARE-name (no `PRGn:`) default-slot file LOAD is now
    IMPLEMENTED to the documented + osb-structural model: a bare name defaults to the RUNNING slot
    (osb `Exec.execute`: `if (!file.hasResourceNumber) file.resourceNumber = currentSlotNumber`),
    so `EXEC "FILE"` / `EXEC EXE$` loads + runs the file in the running slot from the top via
    `Vm::load_into_running_slot` — same path as `EXEC "PRG0:file"` while slot 0 runs. 3 new vm.rs
    e2e tests (bare-name load+run, fresh globals, bare-variable filename). STILL
    VmError::Unsupported / oracle-pending: a `PRG0:` resource naming a NON-running slot 0 (the
    slot-0 registry edge — `load_slot_program` ignores slot 0), and whether real SB's bare-name
    default truly resolves to the running slot vs slot 0 (osb's local `slot` stays 0 in that branch
    — likely an osb bug; needs a ≥2-slot oracle to confirm the running-slot default), plus per-slot
    vs shared variable scoping.)
    (UPDATE 2026-06-23, M6-T6: the slot-0 REGISTRY EDGE is now IMPLEMENTED to the osb-structural
    + documented model — `EXEC "PRG0:file"` / `USE "PRG0:file"` naming a *non-running* slot 0 now
    loads into slot 0 uniformly with the other slots, no longer `VmError::Unsupported`. osb keeps
    all program SLOTs in one `slots[]` array (`VMSlot[5] slots`; `Exec.execute` compiles into
    `slots[slot]` for any slot with no slot-0 carve-out), so sb-core's slot-0 special-casing was
    only an implementation artifact (the running program lives in the VM, parked slots in the
    registry). New `Vm::stash_slot_program` writes a compiled program into ANY parked slot 0..3;
    `do_exec`/`do_use` route a non-running slot 0 through it (the `slot == current_slot` guards
    already keep it from hitting the running program). `EXEC` cross-slot transfers + END-returns to
    its launcher; `USE` marks slot 0 executable so its COMMON DEF resolves via `CALL`. 4 new vm.rs
    e2e tests (EXEC into non-running slot 0 + cross-slot return, own-globals scoping, missing-file
    46, USE slot-0 COMMON DEF callable). STILL ORACLE-PENDING: the slot-0 CLOBBER edge — when an
    EXEC/USE loads into a non-running slot that already holds a pending EXEC-return launcher, the
    launcher's program is overwritten (osb has the same uniform-slot behavior, but the exact
    resume-state real SB preserves across that return needs a ≥2-slot oracle); whether real SB's
    bare-name default resolves to the running slot vs slot 0; per-slot vs shared variable scoping.)
    (UPDATE 2026-06-23, M6-T6 DONE: the BARE-name `USE "file"` (no `PRGn:`) default-slot load is
    now IMPLEMENTED + hw_verified — the LAST `VmError::Unsupported` arm in USE/EXEC is gone. Oracle
    (batch `|err`): `USE "NOPE"`→errnum 4, `USE "Q"`→4, `USE "abc"`→4. A bare name defaults to the
    RUNNING slot (osb `Exec.execute` rule), which you cannot USE → always errnum 4; the running-slot
    guard PRECEDES the file-existence check (a missing bare-name file → 4, not the 46 a missing
    `PRGn:` file gives). `do_use` now resolves `slot_opt.unwrap_or(self.current_slot)` before the
    guard. use.yaml + 1 vm.rs e2e test (`use_string_bare_name_defaults_to_running_slot_errnum_4`).
    All three M6-T6 acceptance criteria met; remaining items below are NON-BLOCKING refinements
    (resume-state granularity, ≥2-slot scoping confirm, slot-0 clobber, callback quirks).)

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
  - MAINCNT frame counter value (monotonic, frames-since-launch — no scalar golden possible).
    RESOLVED 2026-06-23 (M4-T3): `MAINCNT=5` assignment raises **errnum 3** (Syntax error),
    errline 1 — MAINCNT is read-only (the "reset is allowed" rumor is false); corpus shows only
    reads. Still open: boot value / monotonicity across RUN/NEW/CLEAR and halt+CONT.
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
  - SPCHK part RESOLVED 2026-06-24 (spstartstop_rt.tsv): an XY SPANIM gives SPCHK=1 (#CHKXY), so
    `(flags>>17)&0xFF` is confirmed (SPANIM's XY anim-active bit 0x20000 = bit 17 → SPCHK bit 0). The
    BGCHK low-byte mid-animation bit values while a BGANIM channel runs are still oracle-pending (same probe).
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

- M1-T3 · Parser — recursive-descent + precedence + const-fold authored in
  `crates/sb-core/src/parser.rs` (precedence ladder + `constcalc` from osb structurally;
  operator type/wrap semantics from spec/instructions + execution-model.md). A corpus
  parse-sweep over 3,019 small `sbsave` TXT bodies parses ~78% (remainder dominated by
  non-program text files, SB4 BIG programs, and the lexer-level gaps below). Open items
  oracle-/disasm-pending:
  - `#const` in a `DATA` statement (e.g. `DATA #RED,#LIME`, `DATA 30,#WHITE`): the parser
    keeps a `#NAME` as a `Var` marker (compiler resolves it), so it can't fold a `DATA` item
    to a `Lit` — those `DATA` lines currently raise Syntax error. Needs the constant table
    (M1-T5/M1-T7) so `DATA` can resolve `#NAME` to its integer value at compile time.
  - Single-line `IF` extent vs `NEXT`: this parser follows osb — a single-line `IF c THEN …`
    body runs to newline/`ELSE`/`ELSEIF`/`ENDIF`, and a `NEXT` used as a statement is a
    loop-continue (`IF c THEN NEXT` idiom), so `FOR…:IF c THEN x:NEXT` on one line makes the
    `NEXT` part of the IF (FOR then has no terminator). Confirm 3.6.0 single-line-IF extent +
    whether `NEXT`/`WEND`/`UNTIL` as bare statements continue/break vs error.
  - Stray/unbalanced block keywords (a lone `ENDIF`, a multi-line `IF` with no `ENDIF`):
    parser is strict (Syntax error). Confirm whether real SB tolerates an extra `ENDIF` or an
    `IF…THEN`-newline with no `ENDIF` (some shipped programs have these).
  - Lexer-level (M1-T1) gaps surfaced by the sweep, not the parser: `#` as a Double-literal
    suffix on numbers (`0#`, `2#`), scientific-notation literals (`13e4`, `5E2`), and `DATA`
    unquoted strings containing spaces. Queued against M1-T1.
  - `name(a;b;c)` semicolon-separated call args (seen in `DIALOG(…;…;…)`): parser only
    accepts comma-separated call args; confirm whether `;` is legal there or DIALOG-specific.
  - `^` (power) operator: lexer has no caret token and the AST omits it; precedence rank +
    associativity still queued (S-C1/execution-model open item).

## M1-T4 — Value / Array (runtime types) edge cases
- [x] Array **rank mismatch** errnum — RESOLVED hw_verified 2026-06-23 (sb-oracle): a wrong
  subscript COUNT is errnum 3 (Syntax error), genuine out-of-range is errnum 31. `DIM Z[3,2]:
  A=Z[1]`→3, `DIM Z[3]:A=Z[1,1]`→3, `DIM Z[3]:A=Z[3]`→31. Folded into dim.yaml + array.rs.
- **POP/SHIFT on an empty 1D array** errnum: assumed Illegal function call (errnum 4). Confirm
  vs oracle (`DIM A[0]:X=POP(A)` style — note POP/SHIFT are statements/funcs, S-T4b).
- **PUSH/POP/SHIFT/UNSHIFT on a multi-dimensional array** errnum: assumed errnum 4. Confirm
  vs oracle.
- **Double→Integer coercion overflow**: `A%=1E20` / values outside i32 range. value.rs uses
  Rust `f64 as i32` (saturating; ARM VCVT-style). Confirm SB's wrap/saturate behavior vs
  oracle (large + NaN/Inf inputs).

## M1-T5 — Bytecode / Compiler (lowering decisions to confirm)
- **FOR re-evaluation**: the compiler re-evaluates the `TO` and `STEP` expressions every
  iteration (mirrors osb compileFor). Confirm vs oracle whether SB evaluates them once at
  loop entry or each iteration (observable when `TO`/`STEP` reference a variable mutated in
  the body). assumption: re-evaluated each iteration.
- **Auto-declare scope inside DEF**: an undeclared variable first used inside a `DEF` body is
  auto-declared as a **function-local** (execution-model: "variables inside a DEF are local").
  osb auto-declares such reads to a global. Confirm vs oracle whether implicit (non-`VAR`)
  variables inside a DEF are local or global. assumption: local to the DEF.
- **`&&`/`||` result value**: short-circuit ops (`LogicalAnd`/`LogicalOr`) keep the last
  evaluated operand rather than normalizing to 0/1. Confirm SB yields 0/1 vs the operand.
  assumption: last-operand (C-like), no ConvertBool.
- **Suffix-less numeric array default type**: `DIM A[n]` with no suffix makes an Integer-
  element array (matches M1-T4 `default_for_suffix(None)=Int`). But `OPTION DEFINT` "makes the
  default Integer" implies the *non-DEFINT* default is Double. Confirm `DIM A[1]:A[0]=2.7:
  PRINT A[0]` (→2 if Int, →2.7 if Real). assumption: Int element (cross-ref M1-T4 queue).

## M1-T6 — VM (runtime semantics to confirm)
- **Stack-overflow depth (errnum 5)**: the VM caps combined GOSUB + DEF-call depth at
  `CALL_STACK_LIMIT = 8192` (vm.rs) — a hypothesis bound. Confirm the real SB 3.6.0 limit
  (and whether GOSUB and DEF recursion share one stack or have separate limits) via the
  oracle (deeply-nested GOSUB / self-recursive DEF that counts frames before halting).
  Cross-ref the existing execution-model queue line ("recursion depth that trips Stack
  overflow").
- **Shift operators `<<`/`>>`**: vm.rs truncates both operands to i32 then uses Rust
  `wrapping_shl`/`wrapping_shr` (shift count masked to 0..31; `>>` is arithmetic for the
  signed i32). Confirm vs oracle: SB's behavior for shift counts >= 32 and for negative
  shift counts (e.g. `1 << 32`, `1 << -1`, `-8 >> 1`), and whether `>>` is arithmetic or
  logical. assumption: count masked to low 5 bits, arithmetic `>>`.
- **String vs number comparison**: vm.rs raises Type mismatch (errnum 8) for a mixed
  string/number comparison (`"a" == 1`). Confirm vs oracle (SB may instead return false / 0).
- **`&&`/`||` non-normalized result**: short-circuit `LogicalAnd`/`LogicalOr` keep the last
  evaluated operand value (per the compiler lowering), but `Operate(LAnd/LOr)` — emitted only
  if the compiler ever bypasses short-circuit — normalizes to 0/1. Confirm SB's `X=A&&B`
  result value (cross-ref the M1-T5 `&&`/`||` queue entry).

## M1-T7 — Builtins (math/string edges to confirm)
- **STR$/PRINT double formatting** — RESOLVED by M7-T4 (2026-06-23). STR$=C `%g`/6-sig;
  `format_g` reproduces it exactly (round-half-to-even confirmed; verified against a 2000-case
  bit-exact `%.6g` sweep and oracle edges STR$(-0.0)="-0", STR$(0.123456785)="0.123457",
  STR$(0.000000001)="1e-09"). PRINT is DIFFERENT: C `%.8f`+trailing-zero/dot trim
  (`format_print_number`), hw_verified via console read-back (PRINT 12345678.0="12345678",
  PRINT 0.00001="0.00001", PRINT 1.0/3.0="0.33333333", PRINT -0.0="-0"). Still oracle-pending:
  STR$ of subnormals (1.5e-310) and very large magnitudes (1e308) — but the algorithm is the
  C-library `%g`/`%.8f` so these follow deterministically.
- **MIN/MAX of an empty array**: `min_max` returns Illegal function call (errnum 4) for an
  empty array (no element to return). Real SB result unconfirmed — harvest `DIM E[0]:A=MIN(E)`.
- **MID$ negative start/length**: `mid` clamps negative `start`/`length` to 0 (docs only
  cover non-negative). Confirm `MID$("ABC",-1,2)` / `MID$("ABC",1,-1)` vs oracle.
- **SUBST$ start/count past end**: `subst` clamps `start` to len and `count` to `len-start`.
  Confirm `SUBST$("ABC",5,2,"X")` / `SUBST$("ABC",1,9,"X")` vs oracle.
- **VAL parsing details**: `val` trims surrounding whitespace, parses the whole string (else
  0), and accepts `&H`/`&B`/exponent. Confirm leading `+`, leading/trailing whitespace,
  `&H`/`&B` overflow wrap, and lone `"&H"`/`"."` vs oracle. assumption: whole-string parse,
  trim, wrap on overflow.
- **HEX$ digits range**: `hex` rejects `digits` outside 1..63 with Out of range (10), mirroring
  STR$. The spec says only "the supported width range" — confirm the exact HEX$ digits bound.
- **FORMAT$ %B + extras**: `format` supports `%S %D %X %F %B`, flags `-+ 0`, width, `.prec`,
  and `%%`. `%B`, `%%`, unknown-directive passthrough, too-few-args (→ errnum 4 here), and
  type-mismatch-per-directive are oracle-pending (see format.yaml). Harvest a directive sweep.
- **PI()/EXP()/CLASSIFY with-arg-count errors**: arg-count guards (PI with an arg → 4,
  CLASSIFY inf→1/NaN→2) follow the disassembly; harvest to raise to hw_verified.

## M1-T10 — Console model + render (sb-render)
- **Text palette exact ARGB (16 colors)**: `crates/sb-render/src/console.rs TEXT_PALETTE` is
  the documented 16-color set cross-checked vs osb `consoleColor`, quantized to SB 3.6.0's
  hw_verified 5-bit `<<3` device precision (white = `0xF8F8F8` matches hw_verified `#WHITE`).
  The exact per-index text-layer ARGB on 3.6.0 (esp. whether the half-tones are `0x78` like
  the quantized osb `0x7F`, and whether text bypasses 5-bit quantization) is unverified —
  harvest via O-T6 composite screenshot capture (draw a COLOR ramp, screenshot, sample cells).
- **ATTR rotation direction + compose order**: `attr_map` uses clockwise rotation then H/V
  invert. Bit meanings are documented+disassembled (attr.yaml) but the rotation *direction*
  (CW vs CCW) and rotate-then-flip vs flip-then-rotate ordering are oracle-pending — harvest
  by PRINTing an asymmetric glyph under each `#TROT*`/`#TREVH`/`#TREVV` combo and screenshotting.
- **Console font ROM glyphs**: `crates/sb-render/src/font.rs` is a self-contained placeholder
  8×8 font (ASCII subset, lowercase folds to uppercase, no kana/kanji). The real SB glyphs are
  a firmware ROM asset — harvest the font sheet (O-T6) so console goldens can be pixel-matched
  against the emulator instead of being self-consistent only.

## M1-T8 — Control-flow + console builtins
- **PRINT `,` tab vs TABSTEP**: `PrintTab` advances to the next multiple of `tabstep`
  (boot default 4, hardcoded on the VM). Wire TABSTEP as the real writable system variable
  (M6-T3) and harvest a `PRINT a,b` column golden to confirm the tab-stop math + edge wrap.
- **INPUT/LINPUT runtime behavior**: implemented against a headless input queue
  (`Vm::push_input`). NOT modeled vs real SB: the typed-text echo to the console, the
  "?Redo from start" re-prompt on too-few/ill-typed numeric fields (we default an
  unparseable numeric field to 0), and the field/type parsing of mixed numeric+string
  receivers. No deterministic golden (blocks on live keyboard) — error cases only.
- **INKEY$ live keypress**: returns "" headless (no key buffer). A real buffered-key result
  is real-time keyboard state — no deterministic golden; only the empty + arg-count (errnum 4)
  cases are pinned.
- **BACKCOLOR color round-trip + rendered border**: HARVESTED 2026-06-23 (M7-T2 run 2) — the
  GET value is hw_verified: `BACKCOLOR()` returns the stored color masked to 24 bits
  (`& &H00FFFFFF`; the alpha/high byte is dropped, no channel swap). sb-core fixed to mask on
  SET. Only the actual rendered border/clear-color pixel remains screen state (border pixel via
  O-T6).
- **LOCATE Z depth**: the depth operand is range-validated (-256.0..1024.0 → errnum 10) but
  not modeled by the 2-D console grid; z-ordering arrives with the compositor (M2).
- **ACLS full reset**: resets the console color/attr/grid + VM back_color/tabstep here. The
  full documented visual reset (font/sprite/BG reload, both screens, fade/palette) and the
  undocumented 3-arg per-flag selective reset are screen state — oracle-pending (O-T6).

## M1-T14 / arithmetic — constant int*int overflow folds to Double
- **Compile-time int*int overflow promotes to Double on real SB.** Oracle 2026-06-23:
  `2*&H7FFFFFFF` and `2*2147483647` (both constant) → `4.29497e+09` (Double), and
  `2*&H7FFFFFFF==-2` → 0. sb-core's parser constant folder (`fold_binary`) i32-wraps the
  product to `-2` instead. assumption: SB's compile-time folder computes int*int (and
  presumably int+int / int-int) in a wider/Double domain on overflow, while RUNTIME int*int
  still i32-wraps (confirmed: `MAX(A%,3)*&H7FFFFFFF`→2147483641 wraps). Pin the exact folded
  domain (does `1000000*1000000` const → `1e+12` Double? oracle said yes) and whether `+`/`-`
  const overflow promote too, then fix the folder (arithmetic/M7). Does NOT affect otya_test
  (uses the runtime `MAX(…)*…` form). e.g. `?2*&H7FFFFFFF` → 4.29497e+09.

## M1-T5 / execution-model — DEF-local variable scoping  [RESOLVED 2026-06-23]
- **FIXED (M1-T14 increment 2026-06-23).** The rule was pinned via six sb-oracle probes
  (def_scope.yaml): globals (names created by top-level code) ARE visible inside a DEF for
  plain reads/writes (`A=99` inside a DEF overwrites a global A); a `DIM`/`VAR`
  *declaration* inside a DEF binds a fresh function-LOCAL that shadows the same-named
  global; a plain assignment to a name that is not a global creates a local. Compiler fix:
  `compile_dim` now routes through `declare_decl`, which force-binds a local inside a DEF
  (the earlier `lookup`-first path reused the global). Advanced otya 77 → 127.
- **Known residual limitation (static-model divergence, low priority).** A name WRITTEN
  only inside a DEF and READ only at top level (`MKC\nPRINT C\nDEF MKC\n C=7\nEND`) prints
  0 on real SB (the DEF runs first, so `C=7` is local; the later top-level read makes a
  fresh global 0) but 7 in sb-core (the static compiler pre-declares C global from the
  top-level read). Matching needs execution-order dataflow. Does not affect the otya
  pattern (shared globals are WRITTEN at top level first). Documented in def_scope.yaml.

## M1-T14 / SWAP — typed-variable coercion — RESOLVED 2026-06-23
- Fixed: `Op::Swap` now carries each operand's static suffix and re-coerces the swapped-in
  value to the destination's declared type (typed target truncates/widens like an assignment;
  untyped target takes it verbatim). hw_verified by sb-oracle 2026-06-23
  (`A%=1:B#=2.34567:SWAP A%,B# -> A%=2,B#=1`); folded into swap.yaml + harness/corpus/cases/
  swap.yaml. This advanced otya line 127 → 207 (`CALL "SPDEF"`, sprite — M3, the next blocker).

## M1-T14 / block-structure mismatch errnums — RESOLVED 2026-06-23
- Parser raises dedicated structural errnums (was generic Syntax error 3) for unmatched
  control-flow keywords, per the error table (`errors.yaml`, errnum→string ptr table
  @0x3054f8): NEXT without FOR=21, WEND without WHILE=25, UNTIL without REPEAT=23,
  FOR without NEXT=20, WHILE without WEND=24, REPEAT without UNTIL=22, DEF without END=29.
- **All seven CONFIRMED hw_verified** via sb-oracle structural-error `|err` probes 2026-06-23:
  `NEXT`→21, `WEND`→25, `UNTIL 1`→23, `FOR I=0 TO 3`(no NEXT)→20, `WHILE 1`(no WEND)→24,
  `REPEAT`(no UNTIL)→22, `DEF F`(no END)→29. Every one matches the disassembled table AND
  sb-core, so real SB raises THESE (not generic 3). (The batch's first case `NEXT` reported
  errnum 0 once — a bisect artifact; a focused re-probe of `NEXT`/`NEXT I`/`?1:NEXT`/`NEXT:NEXT`
  all returned 21 errline 1.) Sources raised to `hw_verified` in for/while/repeat/wend.yaml
  (next/until/def were already hw_verified for these); consolidated fixture
  `harness/corpus/cases/structural_errnums.yaml` (7 cases) replays them in the hermetic gate.
- Still queued: stray ENDIF/ELSE/THEN (28/27/26) — left as generic 3, unspecced; probe if needed.

## M1-T14 / array-element references — RESOLVED 2026-06-23
- Fixed: `Op::PushArrayRef` is now wired in the VM (was `VmError::Unsupported`). A new
  `Value::ElemRef(ElemRef)` carries the shared array `Rc` + a bounds-checked flat offset
  (resolved at ref time; out-of-range → errnum 31). `deref`/`assign_through` go through the
  element's primitive type (write coerces: `%`→truncate, `#`→widen, `$`→string-or-8). `SWAP`
  rewritten to read-both-then-coerce-both-then-write-both over the generic ref interface, so
  array elements + an aliased SWAP (no-op) both work; `INC`/`DEC` on `A[i]` work via the same
  path. Already hw_verified (swap.yaml s_t4a / inc.yaml s_t4a, sb-oracle 2026-06-22); replayed
  in conformance + harness/corpus/cases/swap.yaml + 9 VM unit tests. Runtime-name refs
  (`Op::PushRefExpr`/`PopRef`, `VAR()`) remain unwired (M6).

## M1-T14 / conformance widening — surfaced gaps queued 2026-06-23
- Widening the conformance allowlist to the full Variables/Data/Console categories surfaced
  these (currently EXCLUDED from `IN_SCOPE_DATA_ARRAY_CONSOLE`; the array-mutation set is in):
  - **VAR duplicate declaration** — RESOLVED 2026-06-23. The compiler now tracks names
    explicitly declared via `DIM`/`VAR` per scope (`Compiler::note_declaration`, a per-scope
    `HashSet<Name>` — `declared_global` + `FuncScope::declared`) and a second declaration of
    the same name (suffix is part of identity) raises errnum 18 (Duplicate variable) at
    compile time. Params and auto-declared (plain-reference) names are NOT tracked, so they
    don't trip it; scopes are independent (a DEF-local `VAR A` doesn't collide with a global
    `VAR A`). hw_verified anchor: sb-oracle 2026-06-22 s_t4a `VAR Q=1:VAR Q=2` → 18. `VAR` is
    now in the conformance allowlist (`IN_SCOPE_DATA_ARRAY_CONSOLE`); var.yaml's inline
    `tests:` (incl. `duplicate_error`) replay green. 3 new compiler unit tests.
  - **LINPUT used as a function** (`A=LINPUT("X")`) — RESOLVED 2026-06-23. `parse_primary`
    now rejects any statement-only command keyword in expression position with a Syntax error
    (3) before the handler runs (the `cur_starts_statement` predicate already marks exactly
    these keywords; `VAR(x)` + word operators are excluded). This fixes `A=LINPUT("X")` → 3
    (was 16; linput.yaml hw_verified s_t5b 2026-06-22) AND the symmetric `A=INPUT("X")` → 3.
    `INPUT`/`LINPUT` are now in the conformance allowlist (`IN_SCOPE_DATA_ARRAY_CONSOLE`) for
    their error inline tests. New `harness/corpus/cases/input_linput.yaml` (3 cases) + 2 parser
    unit tests (`command_keyword_in_expression_position_is_syntax_error`,
    `expression_lookalikes_still_parse`). **Still oracle-pending:** the INPUT *function* form
    `A=INPUT("X")` → 3 is implemented by symmetry but NOT hw_verified (only the literal-receiver
    statement form `INPUT "X";1` → 3 is); confirm `A=INPUT("X")` on the oracle to raise it.
  - **DATA named-constant items** (`DATA #L` → 256) — RESOLVED 2026-06-23. Implemented
    `#NAME` named-constant resolution: the parser now folds every built-in `#NAME` to its
    inline Integer value via a new baked table `sb_core::consts` (all 79 values from the
    hw_verified `spec/reference/constants.yaml`, S-T14c). This fixes `DATA #L`→256 AND bare
    `#UP`/`#WHITE`/… (which previously resolved to 0 as undeclared vars). An UNKNOWN `#NAME`
    keeps the `#`-marker for the compiler. `DATA` is now in `IN_SCOPE_DATA_ARRAY_CONSOLE`;
    data.yaml's `data_named_const` + all DATA inline tests replay green. New
    `harness/corpus/cases/named_const.yaml` (13 cases) + `tests/constants_table.rs` drift
    guard (baked table == constants.yaml) + VM/parser unit tests. (Exact errnum for an
    UNDEFINED `#const` — e.g. `#NOTACONST` — is still oracle-pending; currently it falls
    through to the undeclared-variable path → 0.)
  - **Console output builtins folded in** (M1-T14 increment 2026-06-23): `PRINT`/`COLOR`/
    `CLS`/`INKEY$` (the implemented `Console input/output` builtins, M1-T8) are now in the
    conformance allowlist (`IN_SCOPE_CONSOLE`); their inline `tests:` (origin-printed stdout +
    fg/bg range errnums + empty-INKEY$) replay green. Still EXCLUDED, each its own future
    increment:
    - **LOCATE positioned stdout** (`LOCATE 20,15:PRINT "X"` → `basic_xy`/`x_edge_50_ok`,
      `expect.stdout: "X"`): `console_text()` scrapes the full grid, so the cursor position
      prepends `\n`/spaces — `basic_xy` scrapes to `"\n"*15 + " "*20 + "X"`, and `x_edge_50_ok`
      (`LOCATE 50,0`) scrapes to `"\nX"` because column 50 (a valid LOCATE x — max is 50, not
      49) is past the 50-wide grid's last column (0–49) so the `PRINT` wraps to the next row.
      The column-50 line-wrap is plausible but UNVERIFIED; the value-oracle captures VALUE not
      console text (S-T5a), so neither the positioned whitespace nor the wrap has a golden.
      Harvest the console-grid scrape (screenshot/console-memory path) to confirm, then bake
      the exact positioned expectations and add `LOCATE` to `IN_SCOPE_CONSOLE`.
    - **ATTR/CHKCHR/FONTDEF/SCROLL/WIDTH builtins** (S-T5c) are not implemented in sb-core yet;
      their inline `tests:` (incl. hw_verified errnum 4/10/31 + `CHKCHR`/`WIDTH` value cases)
      fold into `IN_SCOPE_CONSOLE` once those builtins land.

## M3-T5 — BG extras (implemented 2026-06-23; runtime side-effects oracle-pending)
The error gates + form selection are hw_verified (sb-oracle 2026-06-22 s_t9*) and replay in
the conformance gate; the following runtime OUTPUTS need the BG framebuffer/transform oracle
(O-T6) and are implemented to the documented/disassembled behavior pending a harvest:
- **BGANIM interpolation output** — the exact per-frame hold/interpolate values written back
  to a layer's scroll/Z/rot/scale/color/var channel (incl. rounding of the integer channels).
  Implemented via the shared `KeyframeAnim` engine; structural advance is unit-tested.
- **BGANIM channel 2/3 (UV/I) errnum** — BG has no UV/definition-I channel; a numeric target
  2/3 or string "UV"/"I" is currently rejected as Illegal function call (4). The real errnum
  is unverified (the disassembled per-channel switch has no case for them).
- **BGCHK mid-animation bit values** — which `#CHK*` bit is set while a given channel runs
  (need a running BGANIM + the layer flags-word read).
- **BGCOORD converted values** — the exact mode 0/1/2 affine transform output (scroll/rot/
  scale/home). A structural affine is implemented (round-trips with the transforms); the
  pixel-exact values, rotation pivot convention, and char-unit rounding are unverified.
- **BGCOPY out-of-bounds behavior** — cells whose source/destination falls off the map are
  currently skipped (source captured first so overlap is safe); the real clamp/wrap is
  unverified.
- **BGSAVE/BGLOAD cell packing + auto-grow length + trailing-arg** — the packed tile/palette/
  flip cell word format (modeled as the raw 16-bit cell, round-trips within sb-core), the
  auto-grown 1-D array length, and the meaning of the undocumented 3/7-arg trailing operand
  (currently evaluated then ignored) are unverified.

## M3-T6 — Sprite/BG composite into framebuffer (implemented 2026-06-23; pixel-exactness oracle-pending)
The sprite + BG rasterizers and the full layer stack (`compose_top_screen`: backdrop → GRP →
BG×4 → sprites → console, Z-sorted) are wired in `crates/sb-render/src/compositor.rs`. The
**deterministic** behavior — sprite placement at the home point, the 1-bit alpha key, H/V flip,
BG tile placement / scroll / wrap / per-cell H-flip / char-0 transparency, and Z-interleaving
across all layer kinds — is pinned by the compositor unit tests. The following need the
**composite screenshot** capture (O-T6 composite path; `screenshot`/Ctrl+P, NOT the single-page
GRP round-trip) before they can be raised above `hypothesis`:
- **Per-layer default Z + equal-Z tie-break across kinds** — modeled as `GRP < BG < sprite <
  console` (slice order), BG layer 0 (foreground) in front of layer 1+, sprites ascending
  management number = rear→front. Confirm the real paint order (esp. sprite-vs-sprite and
  whether lower or higher SP number is frontmost).
- **Sprite free-rotation / fractional `SPSCALE` sampling** — the inverse-affine nearest-texel
  map here vs SB's exact sub-pixel rule (rounding, pivot handling). Identity / 90°-step / flip
  are exact; arbitrary `SPROT`/`SPSCALE` are not pinned.
- **`SPCHR` sheet offset** — the `chr` field is carried but not yet folded into the source-rect
  sampling; how SPCHR shifts the sampled tile is unverified.
- **Color modulate (`SPCOLOR`/`BGCOLOR`) rounding + whether alpha is modulated**, and the
  **additive (`#SPADD`) blend** math — modeled as `round(src*mod/255)` and saturating RGB add;
  the white/non-additive default (all committed tests) is exact, the rest is a guess.
- **BG 16-color palette remap** — the screen-data palette nibble (bits 12-15) is decoded but
  NOT applied; tiles sample the sheet's RGBA directly. Needs the palette→color mapping.
- **BG sheet tile layout + scroll sign** — char N → sheet tile `(N%(512/tile), N/(512/tile))`
  and `BGOFS` scroll direction (map = screen + ofs) are assumptions; confirm against a capture.

## M4-T1 — Buttons / sticks (BUTTON/STICK/STICKEX/BREPEAT) — live input not headless-harvestable
The oracle has NO input injection (Azahar lacks InputRedirection), so live button magnitudes,
analog axis values, and key-repeat timing cannot be captured deterministically. Modeled to the
disassembled handlers + docs; the no-input/centred baseline + arg/result-count + range error
guards are hw_verified (s_t11a, already in the specs). Queued for a future input-capable oracle
(or hardware capture):
- **BREPEAT default repeat state** — whether SB pre-seeds a non-zero start/interval per button
  at boot. Modeled OFF (feature 1 == feature 2 until BREPEAT sets it). Confirm the default.
- **Key-repeat timing rule** — re-fire modeled as: press fires (raw edge), then after `start`
  frames held the press re-fires, then every `interval` frames. The exact off-by-one (does the
  first repeat land at hold==start or hold==start+1; is the press frame counted) is unverified.
- **STICK/STICKEX axis scale** — raw 16-bit axis × fixed constant, clamped to ±1.0 (docs say
  real extent ≈ ±0.86). The exact scale constant + the centred dead-zone are unpinned.
- **BUTTON wireless terminal form (errnum 52)** — the 2-arg `BUTTON(f,term)` / `STICK term`
  paths raise the undocumented comms error 52 when multiplayer is inactive; gated on live
  wireless, so kept out of the deterministic golden.

## M4-T4 — Display config (XSCREEN/DISPLAY/VISIBLE/HARDWARE) — dual-screen output deferred
The screen *state* (XSCREEN mode, DISPLAY target, VISIBLE per-layer flags, HARDWARE model) is
modeled, with the arg-shape (→ 4) and range (→ 10) guards hw_verified (s_t11d, already in the
specs) and VISIBLE layer gating wired into the compositor (golden-style pixel tests). What is
NOT yet realized / verifiable:
- **DISPLAY 1 → Touch-screen output** — the reimplementation renders only the Upper screen;
  there is no Touch-screen framebuffer/console/GRP, so selecting screen 1 tracks the target but
  does not route console/graphics there. Needs a second-screen render path + the composite
  screenshot capture (O-T6) to verify which screen each layer lands on.
- **XSCREEN sprite/BG split across screens** — the 0..512 / 0..4 allocation is validated but
  not partitioned between Upper and Touch (all sprites/BG composite onto the Upper screen).
- **VISIBLE on the Touch screen + DIRECT-mode guards** — screen-1 visibility is stored but
  unrendered; the XSCREEN-4 / DISPLAY errnum-43 DIRECT-mode guards aren't exercised (programs
  run in program mode, matching the oracle capture). Queued for the DIRECT-mode harness.

## M5-T1 — MML parser (corpus-discovered forms, output-unproven → oracle)
The parser (`crates/sb-audio/src/mml.rs`) accepts these real corpus forms the docs/concept-spec
omit; syntax is proven legal (shipped programs, 541/550 complete BGM* literals parse) but the
runtime semantics are NOT — harvest via sb-oracle (BGMPLAY a probe MML + read back state where
observable, else compare rendered audio by ear / loose spectral per O-T7).
- **`(N` / `)N` volume step with operand** — assumed "change volume by N steps, bare = 1".
  Confirm: is N a step *count* (each step = how many V units?) or an absolute delta? Saturating
  at 0/127? N's ceiling? (`)80` seen.) Seen in 20+ programs (4KHEPXW3/TXT/3DPARKOUR, BGMSET 222).
- **Case-sensitive macro labels** — programs define both `{r=…}` and `{R=…}`; confirm SB3 treats
  them as distinct (parser does) vs. case-folding (would be a redefinition error).
- **Dotted default length `L<n>.`** — `L2.`/`L8.`; confirm a dotted default, and the duration
  when a length-less note then adds its OWN dots (parser sums the default's dots + the note's).
- **Leading accidentals `+B` / `#F` / `-C`** — accidental before the note; confirm same pitch as
  trailing, and whether an accidental before a **rest** (`+R`, `#R`; 1–2 programs) is legal-and-
  ignored or errnum 47 (parser currently → errnum 47).
- **Instrument number ceiling** — corpus shows `@256`–`@411`; parser accepts `@0`–`@511`. Confirm
  the real upper bound and which banks (SFX/drum-kit `@130`–`@134`?) exist in SB3.
- **Default channel state** — tempo 120 / volume 127 / velocity 127 / pan 64 / gate 8 are the
  parser's assumed defaults; confirm against the synth (M5-T2 disasm) + oracle.
- **Tick base** — 192 ticks/whole-note assumed (S-C5); confirm + the T→frames conversion when the
  synth scheduler is read (M5-T2).

## M5-T2 — Synth engine (signal path grounded; voice/curve fidelity = deferred layer per O-T7)
The synth (`crates/sb-audio/src/{synth.rs,instruments.rs}`) renders a parsed `Song` to
interleaved stereo PCM16. Its **signal path is grounded** on the real 3DS DSP via citra/azahar
`audio_core` — native rate 32728 Hz, 160-sample frames, per-voice fractional resample with the
DSP's Q24 linear interpolation + saturated delta (`interpolate.cpp`). The render is fully
deterministic (tested). Per O-T7 there is NO emulator audio golden, so the items below are the
**deferred fidelity layer** — confirm by ear / loose spectral against real SB, never a frozen gate:
- **Instrument sample ROM** — real `@0`–`@127` are sampled GM-equiv voices baked in firmware data
  we don't have; analytic wavetables (Saw/Pulse/Triangle/Sine/Noise) stand in. Extract the SB
  soundbank from romfs to feed real voices through the same resampler (pipeline is already correct).
- **`@E A,D,S,R` envelope curve** — the exact attack/decay/release shape + the param→time mapping
  ("smaller = slower") are placeholders (linear `ENV_MIN_S..ENV_MAX_S`); read the SB synth handler /
  measure.
- **`@V` velocity + `V` + `(`/`)` scaling** — how note velocity scales channel volume (assumed
  multiplicative %); `(`/`)` step size (VOLUME_STEP=1 placeholder); see also M5-T1 `(N`/`)N` item.
- **`@D` detune** — −128..127 → ±2 semitones (`/64`) assumed; confirm the real cents-per-unit.
- **LFO `@MP`/`@MA`/`@ML`** (vibrato/tremolo/autopan) — depth/range/speed/delay → Hz/amount mapping
  is a placeholder; only one active at a time, engaged by `@MON`. Confirm the real curves + `range`/
  `delay` use.
- **Tempo→samples** — `samples/tick = 32728·60/(T·48)`, 48 ticks/quarter (S-C5 192/whole). The
  sample rate is citra-grounded; the SB tick base + exact T→frame quantization want a read of the
  SB synth scheduler in the disassembly (shared with the M5-T1 "Tick base" item).
- **Percussion (`@128`/`@129`) drum map + voices** — pitch→drum mapping (S-C5 table) is not yet
  applied to distinct samples; all percussion is a short noise burst. Needs the drum sample ROM.

### M5-T3 BGM commands — oracle-pending runtime behavior (no deterministic golden, O-T7)
The call shapes + arg ranges + the MML-compile error (47) are disassembled and tested; the
*runtime transport values* below are sb-core's documented assumptions (per the specs) and want
a live SB 3.6.0 read:
- **BGMSETD undefined label** — sb-core raises Undefined label (errnum 14) for `BGMSETD tune,"@L"`
  with no matching DATA block (the RESTORE-shared lookup `bl 0x1ee960`). The `bgmsetd.yaml` `basic`
  case assumes empty stdout; the real errnum/behavior on a missing label is unconfirmed (excluded
  from the conformance gate via `IN_SCOPE_PARTIAL` until harvested). Verify whether a missing label
  errors (14) or silently no-ops.
- **BGMCHK playing value** — the exact non-zero value while a track plays (sb-core returns 1; docs
  say TRUE — could be a richer flag). Stopped → 0 is confirmed-shape.
- **BGMVAR stored/read value while playing** — sb-core stores the written i32 and returns it during
  playback, -1 when stopped (the documented value). The live value mid-tune is unconfirmed.
- **BGMVOL / 3-arg BGMPLAY volume + fade** — the audible mix level / `BGMSTOP track,fade` fade curve
  have no scalar golden.

### M5-T4 SFX / voice — oracle-pending + interpreter-gap notes (no deterministic golden, O-T7)
The call shapes / arg ranges / error conditions are disassembled + tested; the items below are
either unverifiable audio output (O-T7) or a broader interpreter gap to revisit:
- **TALKCHK bare-statement → errnum 3** — a bare `TALKCHK()` used as a statement is rejected at
  parse time with Syntax error (3) on real SB 3.6.0 (function-as-statement). sb-core does not yet
  track function-vs-statement kind, so its handler raises Illegal function call (4) instead. The
  `talkchk.yaml` `bare_statement_syntax_error` case is excluded from the conformance gate via
  `IN_SCOPE_PARTIAL` until function-as-statement parse rejection lands (a parser/compiler feature,
  not M5-specific).
- **WAVSET `[`/`]` repeat groups** — the disassembled hex parser honours bracketed repeat markers
  inside the waveform string (`@0x1a2308`). `decode_waveform` does not expand them (no committed
  case, semantics unverified) and treats a `[`/`]` as a non-hex char → errnum 4. Confirm the exact
  repeat semantics on real SB and implement if a corpus form needs it.
- **BEEP/EFC/WAVSET audible output** — the preset SFX PCM, the reverb preset/raw parameters' audible
  effect, the per-source wet mix, the TTS voice, and the user-instrument waveform synthesis have no
  scalar golden (real-time audio; O-T7). sb-core models only the deterministic state each command sets.
- **WAVSETA reference-pitch / start / end range errors + end<start** — need a live array operand
  (the array-type check precedes them); harvest the errnum for each via the oracle (sb-core: 10/10/10
  for ranges, 4 for end<start, per the disassembly).

### M6-T2 File commands — oracle-pending + interpreter-gap notes
The arg-shape (errnum 3/4) / type (8) / DIRECT-only (44) guards are hw_verified and tested; the
file effects (filesystem state) have no scalar golden but are exercised end-to-end over the
VM-owned in-memory `Storage`. Items to confirm / extend:
- **DAT PCBN byte layout (O-T3)** — sb-core's `SAVE "DAT:"`/`LOAD "DAT:"` use an internal,
  self-describing `"SBDA"` body codec (tag + count + LE elements) so arrays round-trip *within*
  the interpreter. The real SmileBASIC `PCBN0001` element-type tagging (int/double/ushort, array
  dimensions) is queued: until it's pinned, loading a real corpus PCBN `DAT` blob raises Illegal
  file format (35) rather than decoding. Harvest the exact header from a known `SAVE "DAT:"` →
  read-off-disk round-trip (O-T3/O-T4) and replace the codec.
- **Program-slot / GRP / GRPF payloads** — `SAVE`/`LOAD` of a program slot, graphic page or font
  page (form 1) record / require an (empty) resource so existence/FILES/DELETE/RENAME stay
  coherent, but the actual payload (program source text; GRP page bytes) is owned by other
  subsystems and not yet plumbed into the file layer. Wire program source (M6-T4) + GRP pages
  (GSAVE/GLOAD share the page bytes) through `Storage` and confirm the round-trips on real SB.
- **Load-failure errnums 46 / 35** — sb-core maps a missing file → 46 (Load failed) and a
  bad/foreign body → 35 (Illegal file format) per the disassembled `StorageError` map; these are
  documented/disassembled, not yet oracle-confirmed (the `load.yaml` happy/missing-file cases are
  queued — no committed `expect`). Harvest a real missing-file `LOAD` to confirm 46 vs another code.
- **FILES console listing format** — with no output array, sb-core lists one name per line to the
  console. The real on-screen column layout / ordering is unconfirmed (no scalar golden); the
  array-output form (sorted names, 1-D auto-extend) is what the gate asserts.
- **PROGRAM current-slot for bare names** — a bare resource name (`SAVE "NAME"`) targets program
  slot 0 (single-slot M6-T2). Multi-slot routing (the running slot vs. an explicit `PRGn:`) is
  M6-T6; confirm the bare-name → current-slot resolution once multi-slot lands.
- **Cross-resource RENAME (TXT:→PRG:)** — sb-core renames within the *source* resource's folder
  (TXT and program both live in the TXT folder, so the corpus `RENAME "TXT:"+N$,"PRG:"+N$` retype
  works as a same-folder rename). Confirm the real retype semantics on the oracle.

### M6-T3 System variables — oracle-pending + model notes
- **FREEMEM allocator model** — sb-core reports a fixed constant (8314876, the real
  near-empty-program value, sb-oracle 2026-06-23). Real SB FREEMEM *decreases* as a program
  DIMs arrays / defines resources. Modelling the allocator so FREEMEM tracks real usage (and
  the low-memory branch / errnum 4 "Out of memory" boundary) is deferred. Harvest FREEMEM at a
  few known allocation sizes (e.g. `DIM A[100000]:PRINT FREEMEM`) to fit the per-element cost.
- **PRGSLOT default** — the oracle's launch environment read PRGSLOT=1 (the slot the program was
  loaded into). sb-core reports the running slot (0, single-slot M6). The PRG-edit-target slot
  (set by PRGEDIT, M6-T4) and the multi-slot launch slot (M6-T6) decide the real default;
  confirm what a freshly-launched single program reads on the oracle.
- **RESULT semantics** — boots TRUE (1) before any DIALOG (hw_verified 2026-06-23). DIALOG
  (M6-T5) sets TRUE/FALSE/-1 (Suspended). Confirm the exact post-dialog values + the -1
  suspended case on the oracle when DIALOG lands.
- **TABSTEP out-of-range write** — sb-core clamps a negative `TABSTEP=n` to 0 and stores any
  positive value verbatim. The real valid range / whether an out-of-range value raises errnum 10
  (or clamps) is unconfirmed. Harvest `TABSTEP=-1` / `TABSTEP=999` then `PRINT TABSTEP`.
- **SYSBEEP write effect** — sb-core stores the flag and exposes it to the platform UI; whether
  any nonzero is TRUE or only 1 toggles the beep, and whether ACLS resets it, is unconfirmed
  (no audible golden, O-T7). Confirm the truthiness + ACLS-reset behavior on the oracle.

### M6-T4 Source-edit (PRG*) — oracle-pending content + model notes
- **Edited line content + returned text** — PRGEDIT/PRGGET$/PRGSET/PRGINS/PRGDEL round-trip a
  slot's source as a `Vec` of lines in sb-core, verified by vm.rs unit tests. The arg-shape (4),
  slot/type range (10), count-0 (10) and cold-state no-PRGEDIT (38) guards are hw_verified
  (s_t12c/s_t12d); the actual line *text* PRGGET$ returns, the PRGSET append-on-EOF case, and
  the post-op current-line position have no scalar golden in a warm session. Harvest by editing
  a known second slot then SAVEing PRGGET$ output (the PRG* family edits a NON-running slot, so
  a 2-slot oracle program can read it back).
- **PRGSIZE type 1/2 + SLOT_CAPACITY** — type 0 (line count) is faithful; type 1 (characters)
  is modelled as `sum(line.len()+1)` (line text + one LF terminator each) and type 2 (free) as
  `SLOT_CAPACITY - chars` with `SLOT_CAPACITY = 524288` (a placeholder constant). The real
  per-slot character capacity, whether the char count includes line terminators, and the exact
  free-char formula are unconfirmed. Harvest `PRGSIZE(s,1)`/`PRGSIZE(s,2)` on a slot with a
  known source (e.g. `"ABC"+CHR$(10)+"DE"`) to pin the char model + capacity.
- **PRGEDIT -1 / explicit-line bounds** — sb-core treats line `-1` as the last line, allows an
  explicit line in `[0,len]` (len = the append position) and raises errnum 10 past that. The
  exact ARM line-range boundary (the `sub r0,r0,#1; cmp r0,r1; bcc` guard @0x18a240) and the
  empty-slot `-1` result are body-pinned but lack a scalar golden. Confirm on the oracle.
- **PRGNAME$ running/last-run slot name** — the no-arg form reads the running slot's file name
  (current_slot, 0 in single-slot M6). The last-run-slot freeze (STOP/error) and the real LOAD/
  SAVE-set names depend on multi-slot launch (M6-T6); confirm the names a running program reads.
- **Running-slot PRGEDIT guard (errnum 4)** — sb-core raises errnum 4 for `PRGEDIT current_slot`
  (you cannot edit the running slot). Body-pinned (@0x18a1c8) but the warm-session oracle could
  not isolate it as a scalar; confirm `PRGEDIT 0` from a slot-0 program → errnum 4 on the oracle.

### M6-T5 Faithful limitation stubs (XON/MIC/MOTION/MP/DIALOG) — oracle-pending live behavior
The arg-shape (4) / range (10) / type (8) / syntax (3) guards and the XON-MIC (36) / XON-MOTION
(37) availability errors are hw_verified (s_t11b/c, s_t4f) and replay in the conformance gate.
The *live device* outputs have no headless golden — sb-core returns faithful neutral stubs:
- **MICDATA / MICSAVE waveform** — MICDATA returns 0 (8-bit silence ≈ 128 / 16-bit ≈ 0 are the
  documented bases — confirm which a reachable read gives) and MICSAVE writes nothing; with no
  recorded samples a positive MICSAVE count/position raises errnum 10 (count > recorded 0). The
  real sampled values need mic hardware; the in-range vs loop-wrap (errnum 10) split and the
  comms-active errnum 52 path are not headless-harvestable.
- **GYROA/GYROV/ACCEL axes** — sb-core writes 0.0 to all three OUT vars when XON MOTION is on.
  The live radian/G axis values (and the GYROSYNC recalibration) need motion hardware.
- **MP session semantics** — the `@0x305612` restriction flag is treated as 0 (MP reachable in
  DIRECT/program mode, per the oracle running every MP command past it to its arg-count guard).
  Offline (no peers): MPSTART leaves RESULT 0, MPSTAT() = 0, MPRECV = SID -1 / "", and the
  peer-indexed reads (MPSTAT(id)/MPGET/MPNAME$) raise errnum 10 (0 connected terminals). The
  real RESULT/0-1/string values, MPSEND delivery + errnum 41/42 (String too long / buffer
  overflow), and MPRECV/MPNAME$ errnum 11 (Out of memory) need real wireless peers. Also confirm
  whether MPSET truncates or rejects a Double value, and the corpus `MPSET a,b,c` / `MPRECV SID
  OUT RCV$` word-order anomalies (treated as latent program bugs).
- **DIALOG interactive outcome** — headless there is no Touch Screen, so sb-core resolves the
  statement/confirm forms to RESULT 0 (Time out) and the file-name input form to RESULT -1 /
  "" (Canceled). The real 1/-1/0 confirm values, the 128..140 button-detect codes, and the
  entered file-name string need a live Touch Screen (not oracle-harvestable headless).
- **XON confirmation dialog + EXPAD RESULT** — XON shows a one-time confirmation on real SB;
  sb-core flips the feature flag silently and (for EXPAD) sets RESULT TRUE. Confirm the
  already-on no-dialog case and whether XOFF EXPAD clears RESULT.
- **M7-T1 fuzzing — oracle differential + headless VSYNC** — the in-loop campaign is the
  deterministic sb-core robustness sweep (it found + fixed three host panics: GTRI/GCOPY i32
  overflow, VAL char-boundary slice). Still oracle-pending: (a) the 3-way differential
  (sb-core vs osb vs SmileBASIC 3.6.0) over generated seeds to catch *value* divergences, not
  just crashes — wire `harness/diff/run.py --oracle` offline. (b) Confirm the exact SB result
  for the now-clamped extreme-coordinate GTRI/GCOPY (we kept the visible-pixel result identical
  by clamping to the page; verify SB doesn't instead raise an Out-of-range errnum). (c) `VSYNC
  <huge>` blocks headless `sb-run` (broad seeds 403/850) — decide whether headless VSYNC should
  advance instantly; currently it waits, so such programs are parse/compile-only in CI.
- **Sprite coordinates are f32 storage (SPOFS/SPSCALE/SPHOME/SPANGLE…)** — the disassembly
  stores sprite X,Y,Z as 32-bit floats (vldr.32/vstr.32; SPOFS slot+0x30/+0x34/+0x38). sb-core
  stores f64. This is undistinguishable via the oracle's 6-sig-figure `STR$` readback (f32 vs
  f64 agree to ≥7 sig figs for normal magnitudes), so the M7-T2 SPOFS round-trips are all
  exact for both. A value beyond the f32 mantissa (e.g. SPOFS X=16777217 → f32 16777216) WOULD
  diverge and is harvestable via `PRINT` (%.8f, exact) — but the fix is an f32-storage refactor
  spanning every sprite transform setter, not an SPOFS-only slice. Queue: harvest a PRINT-exact
  large-coordinate case per sprite setter, then decide whether to store coords as f32 in
  sb-render's `Sprite`. (SPOFS value/Z-range contract is otherwise hw_verified, M7-T2 run 14.)
- **BG layer scale is f32 storage (BGSCALE)** — same situation as the sprite setters above: the
  disassembly stores the per-layer X/Y scale as a 32-bit float (BGSCALE handler @0x166c4 reads
  back [r0,#0x20] via `vcvt.f64.f32`), while sb-core's `BgLayer.scale_{x,y}` are f64. The M7-T2
  run 17 round-trips (1.5/2.0/0.5/0.4/4/0.25/0/-1, layer-independent, default 1.0) are all exact
  for both representations at the 6-sig-figure `STR$` readback. Only a scale below the f32
  mantissa step (harvestable via `PRINT` %.8f) could distinguish them; folded into the same
  "store transforms as f32" refactor decision as the sprite setters. (BGSCALE value contract is
  otherwise hw_verified, M7-T2 run 17.)
- **SPSET does NOT reset a slot's collision rect+mask (SPCOL), only the scale-adjust flag**
  (sb-core gap, M7-T2 SPCOL run). hw_verified (sb-oracle 2026-06-24): collision state lives in
  a SEPARATE array (slot stride 0x48, indexed by mgmt) distinct from the 2296-byte sprite slot,
  so a second `SPSET m,...` PRESERVES the stored detection rectangle + mask and only clears the
  scale-adjust flag to 0 — `SPSET 0,0:SPCOL 0,7,8,40,48,TRUE,99:SPSET 0,0:SPCOL 0 OUT sx,sy,w,h,sc,mk`
  -> 7,8,40,48,0,99. sb-core's `SpriteState::create` (sb-render sprite.rs) does `..Sprite::default()`,
  wiping ALL collision fields (rect->0, mask->-1, scale->false, enabled->false) on every SPSET.
  Fix = split collision state out of the per-slot `Sprite` into a parallel mgmt-indexed array
  that SPSET leaves alone (clearing only `col_scale_adjust`); spans SPSET/SPCLR/SPCOL/SPHIT*.
  NOT frozen as a conformance case in spcol.yaml (sb-core would fail it); SPCOL's own set-then-read
  value contract IS hw_verified + passing. Queue: implement the collision-array split, then add the
  re-SPSET-preserves-collision conformance case (harvest in harness/harvest/out/spcol_rt2.tsv).
