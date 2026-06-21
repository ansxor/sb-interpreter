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
- SKIP tasks that need the emulator / hardware oracle: the entire `O-` track, and any task
  whose acceptance requires oracle harvest (e.g. capturing golden PNG/WAV). A human runs
  those. If a task is only partly blocked on the oracle, do the parts that aren't, and
  leave it `[ ]` with a progress note.
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
- Implement the task COMPLETELY within its scope. No placeholders, no stubs, no "do later"
  for the part you are doing. (Features that the PRD explicitly defers to a later milestone
  may remain stubbed as documented.)
- Fidelity matters: Integer = i32, Double = f64. Cross-check behavior against `osb/` (D,
  3.5.0 — a hint, never authoritative) and, for exact numbers, the `sb-disassembly/`
  listing. Match SmileBASIC, not Rust defaults.
- Keep `sb-core` free of I/O / GUI / threads (it must build for wasm32). Platform code goes
  in the `sb-platform-*` crates.
- Add or extend DETERMINISTIC tests (fixed RNG seeds, no emulator, no network).

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
  (and update the milestone status table if a whole milestone just completed). Where you
  verified behavior, raise the spec `confidence` and add a `spec/tests/<id>.yaml` overlay.
- If you could NOT finish this run: leave the task `[ ]`, append a short
  "Progress: ..." note under that task's section in `prd/<Mx>.md`, and commit the
  incremental work so the next iteration continues.
- Commit everything: `git add -A && git commit -m "<TASK-ID>: <concise summary>"`.
- Do NOT push, rebase, amend, force, or otherwise rewrite history. One task, one commit.

## 999. Guardrails (the most important rules)
- ONE task per run. Never start a second.
- NEVER mark a task `[x]` unless it is complete AND the entire verification gate is green.
- NEVER run the emulator, fuzzer, or oracle/harvest — they are offline/manual.
- NEVER weaken, skip, or delete a test to make the suite pass. Fix the code instead.
- NEVER touch git history or remotes.
- The task's Acceptance criteria in `prd/<Mx>.md` is the definition of done.
PROMPT_EOF

if [ -f RALPH_PROMPT.md ]; then
  echo "(using RALPH_PROMPT.md override)"
  PROMPT="$(cat RALPH_PROMPT.md)"
fi

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

  # Full stream-json transcript: every assistant message, tool call, tool result, and the
  # final result event. Written straight to disk (not through a pipe) so nothing downstream
  # can SIGPIPE-kill the agent mid-task. `--verbose` is required for stream-json in -p mode.
  printf '%s\n' "$PROMPT" | claude -p --dangerously-skip-permissions --model "$MODEL" \
      --output-format stream-json --verbose >"$jsonl" 2>&1 || true
  ln -sf "$(basename "$jsonl")" "$LOG_DIR/latest.jsonl"

  # Render a readable view of the JSON transcript to console + $log (best effort).
  if command -v jq >/dev/null 2>&1; then
    jq -r '
      if .type=="assistant" then (.message.content[]? |
        if .type=="text" then .text
        elif .type=="tool_use" then "🔧 "+.name+"  "+((.input|tostring)[0:160])
        else empty end)
      elif .type=="result" then
        "\n=== "+(.subtype//"done")+" · turns="+((.num_turns//0)|tostring)
        +" · $"+((.total_cost_usd//0)|tostring)+" · "+((.duration_ms//0)|tostring)+"ms ==="
      else empty end' "$jsonl" 2>/dev/null | tee "$log"
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
