# Ralph loop notes — sb-interpreter

## Last run (2026-06-26) — 6 beads closed, then oracle DIED
- Closed + committed (all hw_verified): o4o (926019e), 1z9 (645f8b5), o2d (c26a929),
  ix5 (7ef7603), 324 (a25ed41), 95p (a1c8b80).
- New bead created: `sb-interpreter-64u` (sb-core bug: SPANIM non-numeric TIME should be
  errnum 34, but `do_spanim` vm.rs:3280 routes time through values_to_f64 → errnum 8).
- Partial (committed fb94eee, bead still OPEN): `sb-interpreter-cco` — BGPUT malformed-hex/
  length/type error model scaffolded as hypothesis/oracle-pending; case file `/tmp/sbcco_cases.txt`
  ready to re-harvest.
- **ORACLE DOWN at end of run.** The 95p `%`-counter endless-loop case
  (`FOR I%=2147483640 TO 2147483647:NEXT`) wedged Azahar; `run_case.py ready` → NOT READY.
  The skill does NOT auto-launch Azahar (needs manual launch + DIRECT-mode setup per
  `.claude/skills/sb-oracle/SKILL.md` Step 0).
- **sb-disassembly listing ALSO not local** this run (`sb-disassembly/` gitignored, ROM not
  on machine) — so `disasm.py` unusable too. Both ground-truth sources were down at end.

## 🚨 NEXT RUN — DO THIS FIRST
1. Launch Azahar manually, get to DIRECT mode, confirm `python3 .claude/skills/sb-oracle/tools/run_case.py ready` → READY.
2. If doing disassembled-confidence work: regenerate/restore `sb-disassembly/listings/cia_3.6.0.lst` + `cia_3.6.0.functions.txt`.
3. NEVER run an endless-loop-risk case via `progcase`/`batch` without a `timeout 30` wrapper — a true hang wedges Azahar and kills the oracle for the whole run. The 95p `%` case should have been `timeout 30 python3 ... progcase 'FOR I%=2147483640 TO 2147483647:NEXT' 'I'` and the None result read as "hangs" without leaving the emulator stuck.

## Closed-bead details this run
- `o4o` — LOCATE Z bounds inclusive at both ends: -256 NOERR, -257 errnum 10, 1024 NOERR, 1025 errnum 10 (matches disasm 0xC3800000/0x44800000). harness/harvest/out/o4o.tsv.
- `1z9` — SPVAR varnum >7 → errnum 10 in ALL 3 forms (setter/function/OUT); guard in FUN_001eec7c. harness/harvest/out/sb1z9.tsv.
- `o2d` — SPANIM non-numeric keyframe ITEM → errnum 8 all forms (XY/Z/inline). BONUS: non-numeric TIME → errnum 34 (Illegal symbol string, NOT 8). harness/harvest/out/sbo2d.tsv.
- `ix5` — GPUTCHR errnum 49 = "Protected resource" (errors.yaml); the plane-availability guard @0x154da4 is UNREACHABLE from user code (GPUTCHR NOERR cold). harness/harvest/out/ix5.tsv.
- `324` — NaN UNREACHABLE in SB 3.6.0: 0/0→errnum 7, SQR(-1)→10, LOG(-1)→10, ACS(2)→3, ATAN(0,0)→error. sb-core to_int_store NaN→0 is a defensive no-op. harness/harvest/out/sb324c.tsv.
- `95p` — FOR counter overflow: suffix-less promotes Int→Double (terminates, 8 passes, I=2.14748e+09=2147483648.0); `%` counter wraps i32 → endless loop. harness/harvest/out/sb95p.tsv.

## Working notes for next runs
- Pick deterministic VALUE-harvest beads (no framebuffer oracle needed) when possible.
- Oracle caveat: `prog 'PRINT #WHITE'` returned None but `progcase 'PRINT #WHITE' '#WHITE'` worked — use progcase for single-statement value probes. errcase works directly for error probes (`errcase '<stmt>'` -> {errored, errnum, errline}).
- HARVEST OUTFILE note: when running run_case.py from the oracle tools dir, the cwd is the tools dir — pass ABSOLUTE out paths or the tsv lands in the skill dir. harness/harvest/out/ is gitignored (transient), but the committed fixture goes into harness/corpus/cases/<name>.yaml (auto-run by conformance.rs).
- Quality gates (Section 4) take ~5min total: fmt+clippy ~10s, builds ~5s, cargo test --workspace ~3min (conformance ~2min of that, in_scope_instruction_specs ~2min, sb-spec ~1min). sb-spec alone is ~50s.
- Conformance harness replays inline `tests:` from `spec/instructions/<id>.yaml` for any id in the IN_SCOPE_* lists (see crates/sb-core/tests/conformance.rs). New error cases auto-run once added.
- Conformance test NAMES are aggregate (in_scope_instruction_specs_pass), not per-case — filter by the aggregate fn name, not case name.

## Good next-bead candidates (P2, oracle-harvestable once Azahar is back up)
- `sb-interpreter-cco` (re-harvest — case file /tmp/sbcco_cases.txt ready): BGPUT malformed hex
  "ZZZZ"/"12GG"/"" /"FF"/"FFFFFF" + >0x2000-char string (→errnum 41?) + array screenData (→errnum 8?).
- `sb-interpreter-64u` (code fix): sb-core do_spanim non-numeric TIME → errnum 34. Spec test
  `nonnumeric_time` is oracle-pending/gated; un-gate after fix. Disasm: time parse likely a
  label/symbol getter (errnum 34 site) vs item value getter (errnum 8 @0x163d98).
- `sb-interpreter-7nc` — S-T7d errnum 49 page-availability guard.
- `sb-interpreter-cjq` — BREPEAT key-repeat delay/rate model.
- `sb-interpreter-wir` — PRGSIZE slot capacity constants (claimed by darienreese).
- `sb-interpreter-mfx` — RNG source (TinyMT/seed).
- `sb-interpreter-air` — stack overflow depth + FREEMEM.
- `sb-interpreter-h5u` — ON (array/expr index).
- `sb-interpreter-9mm` — S-T4d RESTORE.
- `sb-interpreter-q53` — M1-T14 ENDIF leading-comment quirk.
- `sb-interpreter-lvl` — GOTO/GOSUB string-expr target.
- `sb-interpreter-ctm` — parser statement-keyword as bareword value.
- `sb-interpreter-63o` — parser expr-as-statement errnum.

## Beads to AVOID (need framebuffer oracle O-T6, not value harvest — Azahar value path can't do)
- tzn, kyv, m12, m36, tj5, 7nb, zc5, 0nc, 3c1, 9p8, tik, 0p4 (PRINT stdout), 8oe, 4k7, 6an, yuv
- 1ip, bfh (file-format byte verify — needs SAVE+hexdump, doable but slow)

## Workflow that worked
1. `bd ready` -> pick a P2 (not P4 backlog) SPEC-BUILD or oracle-harvest task.
2. `bd show <id>` + read existing `spec/instructions/<id>.yaml`.
3. `bd update <id> --claim`.
4. `python3 .claude/skills/sb-oracle/tools/run_case.py ready`.
5. Harvest via errcase/batch/progcase. ALWAYS pass OUTFILE for batch.
6. Fold result into spec yaml: semantics bullet + hw_verified source + conformance test case.
7. `cargo test -p sb-spec` (gate) + `cargo test -p sb-core --test conformance in_scope_instruction_specs_pass`.
8. `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`.
9. `git add -A && git commit -m "<id>: <summary>"`.
10. `bd close <id> --reason="..."`.
11. Create `.ralph/DONE` to rerun fresh (more beads) or `.ralph/STOP` if none left.

## Kaomoji rule (from ~/.claude/CLAUDE.md)
Every message must begin with a kaomoji. Caveman mode active (fragments ok, code/commits normal).
