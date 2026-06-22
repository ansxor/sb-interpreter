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
#   ./ralph.sh            # loop until all tasks done (or ./ralph.stop appears)
#   ./ralph.sh 5          # run at most 5 iterations
#   RALPH_MODEL=sonnet ./ralph.sh
#
# Stop cleanly: `touch ralph.stop` (consumed once), or Ctrl-C.
# Tune the prompt without editing this file: create RALPH_PROMPT.md (overrides the default).

set -uo pipefail
cd "$(dirname "$0")"

MAX_ITERS="${1:-0}"                       # 0 = unlimited
MODEL="${RALPH_MODEL:-opus}"
LOG_DIR="${RALPH_LOG_DIR:-ralph-logs}"
SLEEP_BETWEEN="${RALPH_SLEEP:-3}"
MAX_NOPROGRESS="${RALPH_MAX_NOPROGRESS:-3}" # stop after N iterations with no commit

command -v claude >/dev/null 2>&1 || { echo "error: 'claude' CLI not found in PATH"; exit 1; }
git rev-parse --git-dir >/dev/null 2>&1   || { echo "error: not a git repository (run: git init)"; exit 1; }
mkdir -p "$LOG_DIR"

cat <<'BANNER'
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
- The sb-oracle skill gives TEXT/VALUE/error ground truth (when Azahar is up), so oracle
  harvest of `expect:`/errnums IS doable in-loop. SKIP only what the skill can't do yet:
  GRAPHICS framebuffer + AUDIO capture (golden PNG/WAV — O-T6/O-T7). If a task is partly
  blocked, do the doable parts and leave it `[ ]` with a progress note.
- If NO task is doable, do not invent work. Write one line to `ralph-logs/BLOCKED.md`
  saying why, then STOP without committing.
- Begin your output with the chosen task ID (e.g. "Picking M1-T1").

## 2. Study the task's brief
- Open the matching milestone doc `prd/<Mx>.md` and read that task's section in full:
  Files, Approach, Acceptance criteria, Deps.
- Read the relevant `spec/instructions/*.yaml` and `spec/reference/*.yaml` entries.
- SEARCH the codebase first to confirm the work isn't already done (don't assume it isn't).
  Reuse existing utilities and patterns.

## 3. Implement — one task only
- **If this is a SPEC-BUILD task (id `S-*`):** your deliverable is the spec FILE(S)
  `spec/instructions/<id>.yaml` (one per instruction in the category), authored to the v2
  contract in `prd/specs.md` from docs + disassembly + osb cross-check — typed signatures
  (ranges/defaults), semantics, error conditions (errnum), and test cases (code → expect).
  Set `confidence` from your source; if Azahar is up, harvest the `expect:` values via the
  sb-oracle skill and set `hw_verified` (commit them), else queue in `HARVEST_QUEUE.md`.
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
- CONFIRM THE ALGORITHM IN THE DISASSEMBLY via the **sb-disasm skill** — MANDATORY: every
  behavior spec must carry a `disassembled` source. Run (from the skill's tools dir):
  `python3 disasm.py find <NAME>` → `disasm.py handler <NAME>` → `disasm.py show <addr> 60`
  to read the handler's ARM/VFP math (rounding mode, int overflow/wrap, float→string format,
  RNG, errnum). `handler` returns CANDIDATES — verify by reading; the right one consumes the
  args and does the relevant math. The disassembly is AUTHORITATIVE for the algorithm —
  consult it even when the oracle gives outputs (it explains WHY + covers edge cases your
  samples miss). Cite the handler (or name) address as a `type: disassembled` source; if the
  handler is index-dispatched and you can't pin it, cite the name address and mark that source
  `confidence: hypothesis`. Integer = i32, Double = f64 — match SmileBASIC, not Rust/osb.
- MANDATORY TESTS: turn the spec's concrete documented values into conformance tests
  (`spec/tests/<id>.yaml` overlays and/or `harness/corpus/cases/*.yaml`) and make sb-core
  pass them. Docs often give exact results (e.g. FLOOR(12.5)=12, FLOOR(-12.5)=-13) — use
  them. A behavior task with NO new spec/corpus test is NOT done. Tests are deterministic
  (fixed seeds, no emulator, no network).
- REAL-PROGRAM CORPUS: `harness/corpus/sbsave/` has 3,329 scraped programs + resources
  (`INDEX.json` manifest; unpack with `python3 tools/extract_sbsave.py`, or fetch one with
  `--get KEY`). Use as test INPUTS — parser/e2e "doesn't panic" sweeps over small
  `type:"TXT"` entries — NEVER as expected output (no oracle = no verified golden). See
  `harness/corpus/sbsave/README.md`.
- Set `confidence` HONESTLY: `documented` (docs), `disassembled` (you read the listing), or
  `hw_verified` (confirmed via the sb-oracle skill AND committed the result).
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
    python3 run_case.py prog 'FLOOR(-2.1)'             # -> -3   (numeric; wraps in STR$)
    python3 run_case.py prog 'MID$("ABCDE",2,3)' str   # -> BCD  (string result)
- It needs Azahar RUNNING with SB on the DIRECT-mode screen (GUI automation). Probe first
  (`python3 sb_window.py bounds`); if it's not up or a case errors, fall back to
  documented/disassembled and queue the case in `HARVEST_QUEUE.md` — do NOT block the task.
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
- A task that implements behavior is NOT done without a new spec/corpus conformance test.
- The task's Acceptance criteria in `prd/<Mx>.md` is the definition of done.
PROMPT_EOF

if [ -f RALPH_PROMPT.md ]; then
  echo "(using RALPH_PROMPT.md override)"
  PROMPT="$(cat RALPH_PROMPT.md)"
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
