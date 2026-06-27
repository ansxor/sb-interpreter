#!/usr/bin/env bash
# ORACLE lane: a serial, one-harvest-at-a-time self-restarting loop (Ralph-style,
# modeled on ~/.local/bin/ralph-claude). Exactly one process — serial by
# construction, which is correct because there is exactly one Azahar oracle.
# The inner claude run drives the loop with sentinels:
#   <state>/DONE  -> finished this harvest, restart fresh for the next bead
#   <state>/STOP  -> queue empty or oracle down: stop the whole loop
set -u
source "$(dirname "$0")/lib.sh"

ORACLE_WT="$WT_BASE/oracle"
SENT_DIR="$STATE_DIR/oracle"; mkdir -p "$SENT_DIR"
DONE_FILE="$SENT_DIR/DONE"; STOP_FILE="$SENT_DIR/STOP"
rm -f "$STOP_FILE"

if [ ! -d "$ORACLE_WT" ]; then
  git -C "$REPO_ROOT" worktree add "$ORACLE_WT" "$LANDING_BRANCH" >/dev/null 2>&1 \
    || { echo "oracle-lane: worktree add failed" >&2; exit 1; }
fi

kill_tree(){ local p="$1" c; for c in $(pgrep -P "$p" 2>/dev/null||true); do kill_tree "$c"; done; kill -TERM "$p" 2>/dev/null||true; }
cleanup(){ if [ -n "${CPID:-}" ] && kill -0 "$CPID" 2>/dev/null; then kill_tree "$CPID"; sleep 2; kill -KILL "$CPID" 2>/dev/null||true; fi; }
trap cleanup INT TERM EXIT

PROMPT="$(cat <<EOF
You are the ORACLE LANE — a serial, one-at-a-time harvest loop. Do EXACTLY ONE harvest this run, then signal the wrapper. Sentinels: create $DONE_FILE to finish and restart fresh for the next bead; create $STOP_FILE to halt the whole loop. Never create both.

Steps:
1. id=\$(bd ready --label $ORACLE_LABEL --json | jq -r '.[0].id // empty'). If empty -> create $STOP_FILE and finish.
2. Confirm the oracle is live: python3 .claude/skills/sb-oracle/tools/run_case.py ready. If NOT READY (Azahar down/crashed) -> bd note "\$id" 'oracle down — human must relaunch Azahar', create $STOP_FILE, finish.
3. bd update "\$id" --claim. Follow the CLAUDE.md SPEC-BUILD protocol: draft/extend the relevant spec/instructions/*.yaml from docs+disassembly if needed, then sb-oracle batch-harvest the expect: values, fold them in as confidence: hw_verified, and add/refresh spec/tests/*.yaml. Run: cargo test -p sb-spec.
4. Stage and commit EXACTLY ONE commit in THIS worktree: git add -A && git commit -m "\$id: <concise summary>".
5. Land it under the merge slot: bash "$ORCH_DIR/integrate.sh" "$ORACLE_WT" "\$id". If it exits 0 -> bd close "\$id". Otherwise -> bd update "\$id" --status open and bd note the failure.
6. Create $DONE_FILE to restart fresh for the next harvest.

Work ONLY in this worktree. Never touch the main branch and never push. If a case cannot be harvested (oracle limitation), leave the spec at documented/disassembled, queue a follow-up bead, and move on.
EOF
)"

while :; do
  rm -f "$DONE_FILE"
  [ -f "$STOP_FILE" ] && { echo "oracle-lane: STOP present; exiting" >&2; exit 0; }
  echo "oracle-lane: starting claude (touch DONE=next, STOP=halt)" >&2
  ( cd "$ORACLE_WT" && claude "$PROMPT" --dangerously-skip-permissions ) &
  CPID=$!
  while kill -0 "$CPID" 2>/dev/null; do
    [ -f "$STOP_FILE" ] && { echo "oracle-lane: STOP seen; halting" >&2; cleanup; exit 0; }
    [ -f "$DONE_FILE" ] && { echo "oracle-lane: DONE seen; restarting" >&2; cleanup; break; }
    sleep 2
  done
  wait "$CPID" 2>/dev/null || true
  [ -f "$STOP_FILE" ] && { echo "oracle-lane: STOP; exiting" >&2; exit 0; }
  [ -f "$DONE_FILE" ] && continue
  echo "oracle-lane: claude exited without DONE/STOP; exiting" >&2; exit 0
done
