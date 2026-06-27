#!/usr/bin/env bash
# IMPLEMENTATION lane dispatcher: single process, hands out beads so there is no
# claim race, caps concurrency at IMPL_CONCURRENCY, spawns one worker per bead.
# Pulls only NON-oracle ready work (oracle-blocked beads are auto-hidden by bd).
set -u
source "$(dirname "$0")/lib.sh"

log "impl dispatch up (concurrency=$IMPL_CONCURRENCY, model=$IMPL_MODEL, exclude-label=$ORACLE_LABEL)"

while :; do
  # concurrency gate (bash 3.2 safe: poll, no `wait -n`)
  while [ "$(running_jobs)" -ge "$IMPL_CONCURRENCY" ]; do sleep 3; done

  id="$(bd ready --exclude-label "$ORACLE_LABEL" --json 2>/dev/null | jq -r '.[0].id // empty')"

  if [ -z "$id" ]; then
    if [ "$(running_jobs)" -gt 0 ]; then
      log "queue empty; draining $(running_jobs) running worker(s)…"; wait; continue
    fi
    log "impl queue empty and idle; exiting"; break
  fi

  bd update "$id" --claim >/dev/null 2>&1 || { log "$id: claim failed, retrying"; sleep 3; continue; }
  log "dispatch -> $id"
  "$ORCH_DIR/impl-worker.sh" "$id" &
done
