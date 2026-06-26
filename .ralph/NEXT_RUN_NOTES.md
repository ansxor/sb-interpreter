# Ralph loop notes — sb-interpreter

## Last run (2026-06-26)
- Bead closed: sb-interpreter-0ka (S-T4b SHIFT multi-dim errnum 8).
- Commit: b59605e. Clean tree.
- Oracle UP + READY (Azahar). `errcase 'DIM A[2,2]:X=SHIFT(A)'` -> errnum 8, errline 1.
- Folded into spec/instructions/shift.yaml: semantics bullet raised to hw_verified, new
  hw_verified source line, new shift_multidim_error conformance case. sb-spec + sb-core
  conformance green.
- `bd ready` shows ~73 remaining P2 beads (not backlog). P4 backlog to SKIP: 3vp, 3lj, c3e.

## Working notes for next runs
- Pick deterministic VALUE-harvest beads (no framebuffer oracle needed) when possible.
- Oracle caveat: `prog 'PRINT #WHITE'` returned None but `progcase 'PRINT #WHITE' '#WHITE'` worked — use progcase for single-statement value probes. errcase works directly for error probes (`errcase '<stmt>'` -> {errored, errnum, errline}).
- HARVEST OUTFILE note: when running run_case.py from the oracle tools dir, the cwd is the tools dir — pass ABSOLUTE out paths or the tsv lands in the skill dir. harness/harvest/out/ is gitignored (transient), but the committed fixture goes into harness/corpus/cases/<name>.yaml (auto-run by conformance.rs).
- Quality gates (Section 4) take ~5min total: fmt+clippy ~10s, builds ~5s, cargo test --workspace ~3min (conformance ~2min of that, in_scope_instruction_specs ~2min, sb-spec ~1min). sb-spec alone is ~50s.
- Conformance harness replays inline `tests:` from `spec/instructions/<id>.yaml` for any id in the IN_SCOPE_* lists (see crates/sb-core/tests/conformance.rs). New error cases auto-run once added.

## Good next-bead candidates (P2, oracle-harvestable now that Azahar is up)
- `sb-interpreter-6v9` (S-T4c) — SORT/RSORT out-of-range start/count errnum. Needs a multi-line prog (PRINT before/after, not bare |err). Likely errnum 10 by analogy to FILL.
- `sb-interpreter-kbv` (S-T4c) — COPY slot-qualified DATA label form. Prior run got errnum 14 for `COPY A,"1:@L"` but corpus shows real usage (13D4DV3V/TXT/MAIN_PRG_V2). Re-harvest with a proper cross-slot setup (load program into slot 1 with @L+DATA, then COPY from slot 0).
- `sb-interpreter-95p` (M7-T5) — FOR counter overflow promotion (suffix-less FOR counter overrunning i32 promotes Int->Double). Derived from INC rule, not yet harvested for FOR. Wrap path risks endless loop — DON'T batch; use a bounded FOR that terminates.

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
