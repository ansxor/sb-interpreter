#!/usr/bin/env bash
#
# ralph.sh — autonomous "Ralph" loop for the SmileBASIC 3.6.0 interpreter.
#
# Inspiration:
#   https://ghuntley.com/ralph/
#   https://github.com/ghuntley/how-to-ralph-wiggum
#
# Each iteration spawns a FRESH `claude -p` agent. Its only memory is the repository:
# PRD.md (the task list), prd/ (per-task briefs), spec/, the code, and git history.
# The agent picks the next doable task, implements ONLY that task, gets the verification
# gate green, checks it off in PRD.md, and commits. Then the loop runs again.
#
# Usage:
#   ./ralph.sh            # headless: loop until all tasks done (or ./ralph.stop appears)
#   ./ralph.sh 5          # headless: run at most 5 iterations
#   ./ralph.sh -i         # interactive: launch the Claude TUI and drive it with /loop
#   ./ralph.sh --sonnet   # use Sonnet instead of the default Opus (alias for --model sonnet)
#   ./ralph.sh --model X  # use any model X
#   RALPH_MODEL=sonnet ./ralph.sh
#
# Two modes — BOTH give a FRESH context per task (single-window drift caused the df691b1 revert):
#   HEADLESS (default) — each iteration spawns a FRESH `claude -p`; the bash loop below picks
#     the task, commits, and repeats. Unattended.
#   INTERACTIVE (`-i` / `--interactive` / RALPH_INTERACTIVE=1) — relaunches a FRESH real Claude
#     TUI for each task (NO `claude -p`). The agent does one task, commits, and `touch
#     .ralph/DONE`; a watcher kills that session and the loop opens a clean one. Watchable +
#     you can intervene live. Permissions are interactive unless RALPH_YOLO=1.
#
# Stop cleanly: `touch ralph.stop` (consumed once), or Ctrl-C.
# Tune the prompt without editing this file: create RALPH_PROMPT.md (overrides the default).

set -uo pipefail
cd "$(dirname "$0")"

# ── Args / flags ────────────────────────────────────────────────────────────────────
INTERACTIVE=0
MODEL="${RALPH_MODEL:-opus}"             # default Opus; RALPH_MODEL or --sonnet/--model override

# Parse flags (any order) before the positional MAX_ITERS. An explicit
# --sonnet/--opus/--model wins over RALPH_MODEL.
while [ $# -gt 0 ]; do
  case "$1" in
    -i|--interactive) INTERACTIVE=1; shift ;;
    -s|--sonnet)      MODEL="sonnet"; shift ;;
    --opus)           MODEL="opus";   shift ;;
    --model)          MODEL="${2:?--model needs a model name}"; shift 2 ;;
    --model=*)        MODEL="${1#--model=}"; shift ;;
    --)               shift; break ;;
    -*)               echo "error: unknown flag '$1'" >&2; exit 1 ;;
    *)                break ;;          # positional (MAX_ITERS) — stop flag parsing
  esac
done
[ "${RALPH_INTERACTIVE:-0}" = "1" ] && INTERACTIVE=1

MAX_ITERS="${1:-0}"                       # 0 = unlimited (headless mode only)
LOG_DIR="${RALPH_LOG_DIR:-ralph-logs}"
SLEEP_BETWEEN="${RALPH_SLEEP:-3}"
MAX_NOPROGRESS="${RALPH_MAX_NOPROGRESS:-3}" # stop after N iterations with no commit

command -v claude >/dev/null 2>&1 || { echo "error: 'claude' CLI not found in PATH"; exit 1; }
git rev-parse --git-dir >/dev/null 2>&1   || { echo "error: not a git repository (run: git init)"; exit 1; }
mkdir -p "$LOG_DIR"

[ "$INTERACTIVE" = 0 ] && cat <<'BANNER'
┌──────────────────────────────────────────────────────────────────────────┐
│ Ralph loop — runs `claude -p --dangerously-skip-permissions` unattended.   │
│ It will edit files and commit on every productive iteration. Ctrl-C or     │
│ `touch ralph.stop` to stop.                                                │
│ Full JSON transcripts in ./ralph-logs/  (live: tail -f ralph-logs/latest.jsonl)
└──────────────────────────────────────────────────────────────────────────┘
BANNER

# ── The prompt handed to each fresh agent (override with RALPH_PROMPT.md) ────────────
read -r -d '' PROMPT <<'PROMPT_EOF'
You are an autonomous coding agent working on the SmileBASIC 3.6.0 interpreter in THIS
repository. You run inside a loop ("Ralph"): every run starts with a FRESH context. Your
only memory is the repository — PRD.md, the prd/ docs, spec/, the source, and git history.
Do EXACTLY ONE task this run, fully and correctly, then commit. Then stop.

## 0. Orient (study — do not skim)
- Study `PRD.md`, the canonical task list. Entries look like:
  `- [ ] M1-T3 — Parser — recursive descent ... → M1-T2`
  where `[x]`=done, `[ ]`=todo, and `→ ID` means "depends on task ID".
- Study `prd/README.md` for the conventions: the confidence ladder, the reference-source
  map (incl. the disassembly UTF-16 / base-0x100000 gotchas), and the coding standards.

## 1. Pick the next task
- Choose the FIRST `- [ ]` task in PRD.md whose every dependency (`→ ID`) is already `[x]`.
- Tasks are SLICED small on purpose (e.g. `S-T1a`, a 3-6 instruction slice). Do exactly ONE
  slice — do NOT widen scope to the whole category. One slice end-to-end beats half a category.
- The sb-oracle skill gives ground truth when Azahar is up: TEXT/VALUE (`batch`), ERRNUM/ERRLINE
  (`errcase` / `|err`), and GRAPHICS (`grp` → PNG; `screenshot` for composite). Harvest these
  in-loop. AUDIO has NO deterministic emulator golden (real-time, timing-dependent) — for
  S-T10/M5 spec the MML grammar + note-events + synth params from docs+disassembly (NO emulator);
  `sb_audio.py` is a manual reference only, not an in-loop golden. If a task is partly blocked, do
  the doable parts and leave it `[ ]` with a progress note.
- If NO task is doable, do not invent work. Write one line to `ralph-logs/BLOCKED.md`
  saying why, `touch .ralph/STOP` (halts the loop), then STOP without committing.
- Begin your output with the chosen task ID (e.g. "Picking M1-T1").

## 2. Study the task's brief
- Open the matching milestone doc `prd/<Mx>.md` and read that task's section in full:
  Files, Approach, Acceptance criteria, Deps.
- Read the relevant `spec/instructions/*.yaml` and `spec/reference/*.yaml` entries.
- SEARCH the codebase first to confirm the work isn't already done (don't assume it isn't).
  Reuse existing utilities and patterns.

## 3. Implement — one task only
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
       mark it oracle-pending and add a line to `HARVEST_QUEUE.md`. NEVER raise `confidence`
       to `hw_verified` from the corpus alone; NEVER overwrite an existing correct claim on
       corpus evidence (a corpus form that seems to contradict the spec is a QUESTION for the
       oracle, not a fact). Keep the top-level `confidence` honest (these additions are
       `community`/oracle-pending, like the existing queued sub-claims).
    3. THEN, if Azahar is up, harvest the `expect:` values — INCLUDING the corpus-surfaced
       cases — via the sb-oracle skill to an OUTFILE (`batch cases.txt out.tsv` — incremental
       + resumable), fold confirmed values in, and raise those sources to `hw_verified`. If
       the run is cut off, what you wrote in steps 1-2 still stands and the OUTFILE holds the
       partial harvest for next time.
    4. Anything not harvested this run: leave `disassembled`/`community` and queue in
       `HARVEST_QUEUE.md`.
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
  documented behavior, add a test, and QUEUE it in `HARVEST_QUEUE.md` (task id · question ·
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
  documented/disassembled + queue in `HARVEST_QUEUE.md` — do NOT block the task.
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
- If the task is FULLY done and the gate is green: flip its `- [ ]` to `- [x]` in PRD.md
  (and update the milestone status table if a whole milestone just completed). Make sure the
  spec `confidence` updates and the new conformance tests from step 3 are committed.
- If you could NOT finish this run: leave the task `[ ]`, append a short
  "Progress: ..." note under that task's section in `prd/<Mx>.md`, and commit the
  incremental work so the next iteration continues.
- Commit everything: `git add -A && git commit -m "<TASK-ID>: <concise summary>"`.
- Do NOT push, rebase, amend, force, or otherwise rewrite history. One task, one commit.
- FINAL ACTION — signal the loop you are done so it can start the NEXT task with a FRESH
  context: after the commit, run `touch .ralph/DONE`. (This is the loop's "end this run"
  signal; do it last, only once your one task is committed. Harmless outside the loop.) If
  you determined NO task was doable, instead `touch .ralph/STOP` so the loop halts.

## 999. Guardrails (the most important rules)
- ONE task per run. Never start a second.
- NEVER mark a task `[x]` unless it is complete AND the entire verification gate is green.
- The deterministic gate (`cargo test`) stays hermetic — it never needs the emulator. DO use
  the sb-oracle skill to harvest ground truth, freezing results into committed fixtures.
  Don't run the fuzzer in-loop.
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
  word-boundary re-grep before amending, record it as `community` confidence + an
  `HARVEST_QUEUE.md` line (never `hw_verified`), and never overwrite a correct claim on
  corpus evidence alone.
- A task that implements behavior is NOT done without a new spec/corpus conformance test.
- The task's Acceptance criteria in `prd/<Mx>.md` is the definition of done.
PROMPT_EOF

if [ -f RALPH_PROMPT.md ]; then
  echo "(using RALPH_PROMPT.md override)"
  PROMPT="$(cat RALPH_PROMPT.md)"
fi

# ── Interactive mode: relaunch a FRESH Claude TUI per task (no `claude -p`) ─────────────
# Each task runs in a brand-new `claude "<prompt>"` session, so context NEVER persists
# across tasks (that single-window drift is what produced the df691b1 revert — the model
# stopped reading the disassembly and self-reinforced the shortcut). The agent does ONE task,
# commits, and `touch .ralph/DONE` as its last action; a watcher then kills that session and
# the loop relaunches a clean one. No `-p` is used — this is the real interactive TUI.
# Stop: the agent `touch .ralph/STOP` when nothing's doable, or `touch ralph.stop`, or Ctrl-C.
if [ "$INTERACTIVE" = 1 ]; then
  YOLO=""
  [ "${RALPH_YOLO:-0}" = "1" ] && YOLO="--dangerously-skip-permissions"
  mkdir -p .ralph
  echo "┌──────────────────────────────────────────────────────────────────────────┐"
  echo "│ Interactive Ralph — FRESH context per task (model=$MODEL), no claude -p.    │"
  echo "│ One task per session; the loop relaunches a clean TUI after each commit.    │"
  [ -n "$YOLO" ] && echo "│ Permissions: BYPASSED (RALPH_YOLO=1)." \
                 || echo "│ Permissions: interactive (shift+tab in the TUI to change mode)."
  echo "│ Stop: agent writes .ralph/STOP, or 'touch ralph.stop', or Ctrl-C.          │"
  echo "└──────────────────────────────────────────────────────────────────────────┘"

  iter=0
  noprogress=0
  while :; do
    rm -f .ralph/DONE .ralph/done .ralph/STOP Ralph.done ralph.done
    [ -f ralph.stop ] && { echo "ralph.stop found — stopping."; rm -f ralph.stop; break; }
    if ! grep -q '^- \[ \] ' PRD.md; then
      echo "🎉 All PRD tasks are checked off. Nothing left to do."; break
    fi
    iter=$((iter + 1))
    if [ "$MAX_ITERS" -gt 0 ] && [ "$iter" -gt "$MAX_ITERS" ]; then
      echo "Reached MAX_ITERS=$MAX_ITERS."; break
    fi
    remaining="$(grep -c '^- \[ \] ' PRD.md)"
    echo "──────── task $iter ── model=$MODEL ── $remaining left ── fresh context ────────"
    before="$(git rev-parse HEAD 2>/dev/null || echo none)"

    # Watcher: when the agent drops a done/stop sentinel, terminate THIS wrapper's
    # Claude child process tree. $$ is the wrapper PID; $BASHPID is the watcher's own.
    # Accept legacy root-level Ralph.done/ralph.done too, in case Claude writes the
    # wrong sentinel name; the canonical signal remains .ralph/DONE.
    ( self=$BASHPID
      sentinel_seen() {
        [ -f .ralph/DONE ] || [ -f .ralph/done ] || [ -f .ralph/STOP ] || [ -f Ralph.done ] || [ -f ralph.done ]
      }
      kill_tree() {
        local pid="$1"
        local child
        for child in $(pgrep -P "$pid" 2>/dev/null); do
          kill_tree "$child"
        done
        kill -TERM "$pid" 2>/dev/null || true
      }
      kill_tree_hard() {
        local pid="$1"
        local child
        for child in $(pgrep -P "$pid" 2>/dev/null); do
          kill_tree_hard "$child"
        done
        kill -KILL "$pid" 2>/dev/null || true
      }
      while ! sentinel_seen; do sleep 2; done
      for p in $(pgrep -P $$ 2>/dev/null); do
        [ "$p" != "$self" ] && kill_tree "$p"
      done
      sleep 2
      for p in $(pgrep -P $$ 2>/dev/null); do
        [ "$p" != "$self" ] && kill_tree_hard "$p"
      done ) &
    watcher=$!

    # Fresh interactive session for exactly one task. Killed by the watcher above.
    claude --model "$MODEL" $YOLO "$PROMPT"

    kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null

    if [ -f .ralph/STOP ]; then echo "agent signalled STOP — halting."; rm -f .ralph/STOP; break; fi

    after="$(git rev-parse HEAD 2>/dev/null || echo none)"
    if [ "$before" = "$after" ]; then
      noprogress=$((noprogress + 1))
      echo "⚠ no commit this task ($noprogress/$MAX_NOPROGRESS)."
      [ "$noprogress" -ge "$MAX_NOPROGRESS" ] && { echo "No progress for $MAX_NOPROGRESS tasks — stopping."; break; }
    else
      noprogress=0
      echo "✓ committed: $(git log -1 --oneline)"
    fi
    sleep "$SLEEP_BETWEEN"
  done
  exit 0
fi

# jq filter that renders the stream-json transcript into a readable line-per-event view.
# Used both for the live stdout tail (during the run) and the persisted $log (after).
read -r -d '' RENDER_FILTER <<'JQ_EOF' || true
if .type=="assistant" then (.message.content[]? |
  if .type=="text" then .text
  elif .type=="tool_use" then "🔧 "+.name+"  "+((.input|tostring)[0:160])
  else empty end)
elif .type=="result" then
  "\n=== "+(.subtype//"done")+" · turns="+((.num_turns//0)|tostring)
  +" · $"+((.total_cost_usd//0)|tostring)+" · "+((.duration_ms//0)|tostring)+"ms ==="
else empty end
JQ_EOF

# ── Loop ─────────────────────────────────────────────────────────────────────────────
iter=0
noprogress=0
while :; do
  if [ -f ralph.stop ]; then echo "ralph.stop found — stopping."; rm -f ralph.stop; break; fi

  if ! grep -q '^- \[ \] ' PRD.md; then
    echo "🎉 All PRD tasks are checked off. Nothing left to do."
    break
  fi

  iter=$((iter + 1))
  if [ "$MAX_ITERS" -gt 0 ] && [ "$iter" -gt "$MAX_ITERS" ]; then
    echo "Reached MAX_ITERS=$MAX_ITERS."
    break
  fi

  ts="$(date +%Y%m%d-%H%M%S)"
  stem="$LOG_DIR/iter-$(printf '%03d' "$iter")-$ts"
  jsonl="$stem.jsonl"   # full structured transcript (the durable record)
  log="$stem.log"       # human-readable rendering
  remaining="$(grep -c '^- \[ \] ' PRD.md)"
  echo "──────── iteration $iter ── model=$MODEL ── $remaining task(s) left ── $ts"
  echo "   full JSON → $jsonl   (live: tail -f $LOG_DIR/latest.jsonl)"

  before="$(git rev-parse HEAD 2>/dev/null || echo none)"

  : > "$jsonl"                                       # create now so the live tailer can follow it
  ln -sf "$(basename "$jsonl")" "$LOG_DIR/latest.jsonl"

  # Live progress: follow the transcript as the agent writes it and render to stdout in real
  # time. This is a READ-ONLY tail of a growing file — it can't SIGPIPE the agent, which still
  # writes straight to disk below.
  live_pid=""
  if command -v jq >/dev/null 2>&1; then
    ( tail -n +1 -f "$jsonl" 2>/dev/null | jq --unbuffered -r "$RENDER_FILTER" 2>/dev/null ) &
    live_pid=$!
  fi

  # Full stream-json transcript: every assistant message, tool call, tool result, and the
  # final result event. Written straight to disk (not through a pipe) so nothing downstream
  # can SIGPIPE-kill the agent mid-task. `--verbose` is required for stream-json in -p mode.
  printf '%s\n' "$PROMPT" | claude -p --dangerously-skip-permissions --model "$MODEL" \
      --output-format stream-json --verbose >"$jsonl" 2>&1 || true

  # Stop the live tailer (give it a beat to flush the final lines first).
  if [ -n "$live_pid" ]; then
    sleep 1
    pkill -P "$live_pid" 2>/dev/null   # the tail + jq children of the subshell
    kill "$live_pid" 2>/dev/null       # the subshell itself
    wait "$live_pid" 2>/dev/null
  fi

  # Persist a readable rendering of the transcript to $log (console already saw it live).
  if command -v jq >/dev/null 2>&1; then
    jq -r "$RENDER_FILTER" "$jsonl" 2>/dev/null > "$log"
  else
    cp "$jsonl" "$log"; tail -n 20 "$jsonl"
  fi

  # Backstop: guarantee a commit per productive iteration even if the agent forgot to.
  if [ -n "$(git status --porcelain)" ]; then
    git add -A
    git commit -q -m "ralph(iter $iter): autocommit leftover changes" || true
  fi

  after="$(git rev-parse HEAD 2>/dev/null || echo none)"
  if [ "$before" = "$after" ]; then
    noprogress=$((noprogress + 1))
    echo "⚠ no commit this iteration ($noprogress/$MAX_NOPROGRESS)."
    if [ "$noprogress" -ge "$MAX_NOPROGRESS" ]; then
      echo "No progress for $MAX_NOPROGRESS iterations — stopping. See $LOG_DIR/ and ralph-logs/BLOCKED.md."
      break
    fi
  else
    noprogress=0
    echo "✓ committed: $(git log -1 --oneline)"
  fi

  sleep "$SLEEP_BETWEEN"
done
