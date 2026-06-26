# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this project.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:6cd5cc61 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->


## Build & Test

_Add your build and test commands here_

```bash
# Example:
# npm install
# npm test
```

## Architecture Overview

_Add a brief overview of your project architecture_

## Conventions & Patterns

_Add your project-specific conventions here_

TESTS TAKE A REALLY LONG TIME TO RUN. be conservation when to run them, because it takes 5 minutes to run
all of them, and it causes a lot of friction.

## Implementing
- **If this is a SPEC-BUILD task (id `S-*`):** your deliverable is the spec FILE(S)
  `spec/instructions/<id>.yaml` (one per instruction in the slice), authored to the v2
  contract in `prd/specs.md` from docs + disassembly + osb cross-check — typed signatures
  (ranges/defaults), semantics, error conditions (errnum), and test cases (code → expect).
  **PERSIST FIRST, MINE THE CORPUS, HARVEST LAST** (the oracle is slow and a run can be cut
  off mid-harvest):
    1. Write the COMPLETE spec from docs + disassembly + osb, with `expect:` filled from the
       docs/disassembly and `confidence: disassembled`. This is already valuable + commit-able
       on its own — never gate it behind a slow oracle pass.
    2. CORPUS EDGE-CASE SWEEP — catch real forms the docs never mention. For EACH instruction
       in the slice, spawn a Haiku subagent (`Agent` tool, `model: haiku`) to grep its real
       usage in `harness/corpus/sbsave/files/*/TXT/*` (3,329 decoded programs) and report
       undocumented / edge forms: extra optional args, alternate syntaxes, string-var vs
       literal operands, implicit forms, array vs scalar, contradicted claims. Tell each agent
       to return CANDIDATES ONLY (pattern + one example `KEY/TXT/NAME` + rough count) — NOT
       edits. THEN verify every candidate YOURSELF before believing it: re-grep with WORD
       BOUNDARIES (`rg '\bNEXT [A-Z]'`, never a bare substring — `NEXT` matches inside
       `SET_NODE_NEXT`/`EQUNEXT`) and discard substring/false-positive hits. The corpus are
       real shipped programs, so a VERIFIED form PROVES the SYNTAX is legal — but it does NOT
       prove runtime output. For each real, currently-missing form: add/extend a `semantics`
       bullet (and a signature/test if apt), cite a `type: community` source
       (`sbsave corpus: <form> in N programs, e.g. <KEY>/TXT/<NAME>`), and — output unproven —
       mark it oracle-pending and add it to beads. NEVER raise `confidence`
       to `hw_verified` from the corpus alone; NEVER overwrite an existing correct claim on
       corpus evidence (a corpus form that seems to contradict the spec is a QUESTION for the
       oracle, not a fact). Keep the top-level `confidence` honest (these additions are
       `community`/oracle-pending, like the existing queued sub-claims).
    3. THEN, if Azahar is up, harvest the `expect:` values — INCLUDING the corpus-surfaced
       cases — via the sb-oracle skill to an OUTFILE (`batch cases.txt out.tsv` — incremental
       + resumable), fold confirmed values in, and raise those sources to `hw_verified`. If
       the run is cut off, what you wrote in steps 1-2 still stands and the OUTFILE holds the
       partial harvest for next time.
    4. Anything not harvested this run: leave `disassembled`/`community` and make a new
       task in beads with the related information.
  Write NO interpreter code. Verify with `cargo test -p sb-spec`, then commit. (The rest of
  section 3 is for code tasks.)
- SPEC-FIRST: the contract is the spec (`spec/instructions/<id>.yaml` + `spec/reference/*`)
  and the task's Acceptance criteria — what SmileBASIC 3.6.0 does per the docs. Implement
  to the spec, not to osb.
- `osb/` (D, 3.5.0) is a STRUCTURAL reference ONLY — consult it for how to shape a
  lexer/parser/VM, NEVER as the definition of behavior. Do NOT translate it line-by-line,
  do NOT copy its comments, and do NOT reproduce its limitations or 3.5.0-isms (example:
  osb lexes ASCII-only identifiers, but SmileBASIC is Japanese and allows full-width/kana
  names). Where osb disagrees with the docs/disassembly, the docs/disassembly win.
- CONFIRM THE ALGORITHM IN THE DISASSEMBLY via the **sb-disasm skill** — and `disassembled`
  means you READ THE HANDLER BODY, not that you looked up its address. This is the #1 way
  specs get reverted (see commit df691b1: 14 slices cited the dispatch address, wrote
  plausible prose from the docs, and labeled it `disassembled` — all reverted). The hard rule:
    * `python3 disasm.py dispatch <NAME>` gives the AUTHORITATIVE handler address AND now
      auto-prints the first lines of the handler body + a paste-ready ref skeleton. The
      address alone is NOT a citation.
    * Before ANY source is labeled `type: disassembled`, you MUST have run `disasm.py show
      <addr> 60` (or `showmany` for several — ONE call, not a bash for-loop) and your ref
      MUST quote real listing detail from that body: a concrete errnum site (`mov r0,#0x4`),
      a range guard (`vcmpe …`), a constant, a rounding/overflow step, or ≥2 real addresses
      (handler + an internal helper/const). A ref that is only an address + behavioral prose
      is FORBIDDEN — label it `confidence: hypothesis` instead. The spec gate enforces this:
      `cargo test -p sb-spec` (test `disassembled_sources_show_evidence_of_a_body_read`) FAILS
      a `disassembled` ref with no body evidence and a mismatched `FUN_…/@0x…`, so a faked one
      will not pass step 4.
    * The disassembly is AUTHORITATIVE for the algorithm — consult the body even when the
      oracle gives outputs (it explains WHY + covers edge cases your samples miss).
  Operators/special forms (AND/OR/MOD/PRINT/PI…) aren't in the dispatch table — `dispatch`
  says so; use `disasm.py handler <NAME>` (heuristic) / `find` + `xref`, and if you still can't
  pin the body, cite the name address as `confidence: hypothesis` (NOT `disassembled`).
  Integer = i32, Double = f64 — match SmileBASIC, not Rust/osb.
- MANDATORY TESTS: turn the spec's concrete documented values into conformance tests
  (`spec/tests/<id>.yaml` overlays and/or `harness/corpus/cases/*.yaml`) and make sb-core
  pass them. Docs often give exact results (e.g. FLOOR(12.5)=12, FLOOR(-12.5)=-13) — use
  them. A behavior task with NO new spec/corpus test is NOT done. Tests are deterministic
  (fixed seeds, no emulator, no network).
- REAL-PROGRAM CORPUS: `harness/corpus/sbsave/` has 3,329 scraped programs + resources
  (`INDEX.json` manifest; unpack with `python3 tools/extract_sbsave.py`, or fetch one with
  `--get KEY`; decoded source at `files/<KEY>/TXT/*`). TWO uses: (a) test INPUTS — parser/e2e
  "doesn't panic" sweeps over small `type:"TXT"` entries; (b) EDGE-CASE DISCOVERY for spec
  work — grep an instruction's real usage to surface undocumented forms (see the SPEC-BUILD
  corpus sweep above). It is a DISCOVERY source, NEVER a verified golden: a corpus form proves
  the syntax is legal but not its output (no oracle = no `hw_verified`, no `expect:` golden).
  See `harness/corpus/sbsave/README.md`.
- Set `confidence` HONESTLY: `documented` (docs), `disassembled` (you read the handler BODY
  with `disasm.py show` and your ref quotes real listing detail), or `hw_verified` (confirmed
  via the sb-oracle skill AND committed the result). If you only have the address, it is
  `hypothesis`, not `disassembled`.
- If a 3.6.0 edge case is NOT determinable from docs/disassembly: prefer harvesting it via
  the sb-oracle skill (then it's `hw_verified`). If the oracle isn't available, implement the
  documented behavior, add a test, and QUEUE it in beads (task id · question ·
  your assumption). Never silently inherit an unverified behavior from osb.
- Keep `sb-core` free of I/O / GUI / threads (must build for wasm32); platform code goes in
  the `sb-platform-*` crates.

### Ground truth: the sb-oracle skill (real SmileBASIC 3.6.0)
The `.claude/skills/sb-oracle/` skill drives REAL SB 3.6.0 in Azahar — it IS the ground-truth
oracle. Use it to (a) HARVEST `hw_verified` expects for spec/test cases and (b) differentially
check that `sb-core`'s output matches real SB. From `.claude/skills/sb-oracle/tools/`:
    python3 run_case.py ready                          # FIRST: launch Azahar if needed + probe -> READY
    python3 run_case.py batch cases.txt out.tsv        # FAST harvest: ONE mega-program for all cases
    python3 run_case.py prog 'FLOOR(-2.1)'             # one case -> -3
- FIRST run `run_case.py ready` (it launches Azahar + confirms SB is usable). Then `batch` your
  slice's cases — ONE process, NO backgrounding/sleep (the harness blocks `sleep N; cmd`).
  `batch` is FAST: it writes ONE program that evaluates all value cases and SAVEs them in a
  single file (≈one LOAD+RUN, not one-per-case), and bisects around any case that halts (SB has
  no error trapping). Case lines: `name|expr`, `name|expr|str` (string result), `name|stmt|err`
  (a statement EXPECTED to raise → captures ERRNUM/ERRLINE; runs alone), or bare `expr`. Use
  `|err` for error-expecting spec tests (e.g. `sqr_neg|X=SQR(-1)|err`).
  ALWAYS pass an OUTFILE: each result is written + flushed as it resolves, so if this run is cut
  off the partials survive and re-running `batch` with the same OUTFILE skips done cases and
  retries only failures. If `ready` says NOT READY or a case errors, fall back to
  documented/disassembled + queue in beads — do NOT block the task.
- The oracle result is the SOURCE OF TRUTH: if `sb-core` disagrees, `sb-core` is wrong.
- When you get an oracle result, write it into the spec's `spec/tests/<id>.yaml` `expect:`
  (and/or `harness/corpus/cases`), set that source `confidence: hw_verified`, and COMMIT it.
  It's now a frozen fixture the deterministic gate replays forever WITHOUT the emulator.
- Don't re-harvest a case that already has a committed `hw_verified` expect.

## 4. Verify — must be fully green before you mark a task done
Run these and make them ALL pass:
    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo build --workspace
    cargo build --workspace --target wasm32-unknown-unknown
    cargo test --workspace
If you changed Python, also: `python3 -m py_compile` the changed files; if you changed
`tools/gen_specs.py`, re-run it and ensure `spec/` does not drift.

## 5. Record + commit — ALWAYS finish with a commit
- Commit everything: `git add -A && git commit -m "<TASK-ID>: <concise summary>"`.
- Do NOT push, rebase, amend, force, or otherwise rewrite history. One task, one commit.

## 999. Guardrails (the most important rules)
- NEVER weaken, skip, or delete a test to make the suite pass. Fix the code instead.
- NEVER touch git history or remotes.
- NEVER write a line-by-line osb port or inherit osb's limitations — implement to the spec;
  osb is a structural hint only. Don't even write "port of osb" in comments; describe the
  3.6.0 behavior you implemented and cite the spec/disassembly.
- Set `confidence: hw_verified` ONLY from a committed sb-oracle result — never guess it.
- NEVER label a source `disassembled` from a dispatch address + docs prose. `disassembled`
  REQUIRES running `disasm.py show <addr>` and quoting real body detail (errnum site / range
  guard / constant / ≥2 addresses). No body read → `hypothesis`. (This is what got commit
  df691b1's 14 slices reverted; `cargo test -p sb-spec` now fails the build if you fake it.)
- The sbsave corpus is a DISCOVERY source only: verify every grepped candidate with a
  word-boundary re-grep before amending, record it as `community` confidence + a
  bead line (never `hw_verified`), and never overwrite a correct claim on
  corpus evidence alone.
- A task that implements behavior is NOT done without a new spec/corpus conformance test.
