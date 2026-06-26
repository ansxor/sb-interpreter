# Ralph handoff — next run notes

## Last run (2026-06-26)
- Completed `sb-interpreter-kbv` (S-T4c: COPY slot-qualified DATA label form).
  - Harvested via sb-oracle: `COPY A,"1:@L"` works when slot 1 is loaded + USE 1 +
    label exists. Prior errnum-14 was correct (missing USE 1 / no slot-1 program).
  - Spec `spec/instructions/copy.yaml` updated (signature + semantics + hw_verified
    source). Harvest tool: `.claude/skills/sb-oracle/tools/slotcopy_harvest.py`.
    Provenance: `harness/harvest/out/slotcopy_t4c.tsv` (gitignored — regenerable).
  - Committed as `0fa4a81`. Bead closed. Follow-up `sb-interpreter-6av` filed
    (sb-core impl + runnable cross-slot conformance test; blocked on M6-T6 multi-program).
- Quality gates: `cargo fmt`, `cargo build --workspace`, `cargo test --workspace`,
  `cargo test -p sb-spec` all green.

## How to pick the next bead
- `bd ready` — 71 ready P2 issues, none in backlog. Pick a self-contained one.
- Oracle (sb-oracle) is UP — value-harvest tasks work well:
  - `sb-interpreter-wir` PRGSIZE slot capacity (scalar harvest)
  - `sb-interpreter-mfx` RNG TinyMT seed model (seeded RND sequence harvest)
  - `sb-interpreter-cjq` BREPEAT (live-keyboard — no golden; spec + disasm only)
  - `sb-interpreter-bfh` GLOAD/GSAVE ushort PCBN DAT body tag (file-format probe)
- Always: `bd update <id> --claim`, do the work, `bd close <id>`, run quality gates,
  commit as `<id>: <summary>`. Do NOT push/sync (conservative profile).
- If oracle needed: `python3 .claude/skills/sb-oracle/tools/run_case.py ready` first.

## Stop condition
- If `bd ready` shows only backlog (P4) or nothing, create `.ralph/STOP`.
