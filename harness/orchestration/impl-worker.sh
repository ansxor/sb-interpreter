#!/usr/bin/env bash
# IMPLEMENTATION lane worker: one bead, isolated worktree, omp+model brain,
# tier-1 cheap gates in parallel, tier-2 land under the merge slot.
#   impl-worker.sh <bead-id>
set -u
source "$(dirname "$0")/lib.sh"

id="${1:?usage: impl-worker.sh <bead-id>}"
title="$(bd show "$id" --json 2>/dev/null | jq -r '.title // .summary // .name // empty')"
stamp="$(date +%s)"
wt="$WT_BASE/worker-$id-$stamp"
br="worker-$id-$stamp"

reopen(){ bd update "$id" --status open >/dev/null 2>&1; [ -n "${1:-}" ] && bd note "$id" "$1" >/dev/null 2>&1; }
cleanup(){ git -C "$REPO_ROOT" worktree remove --force "$wt" >/dev/null 2>&1; git -C "$REPO_ROOT" branch -D "$br" >/dev/null 2>&1; }
trap cleanup EXIT

git -C "$REPO_ROOT" worktree add -b "$br" "$wt" "$LANDING_BRANCH" >/dev/null 2>&1 \
  || { log "$id: worktree add failed"; reopen "impl-lane: worktree add failed"; exit 1; }

prompt="You are an IMPLEMENTATION worker in the sb-interpreter repo. Implement beads issue $id ($title).
- The spec is the contract: read spec/instructions/<the instruction>.yaml and 'bd show $id'.
- Write sb-core code AND at least one conformance test (spec/tests/*.yaml overlay or harness/corpus/cases/*.yaml). A behavior change with no new test is NOT done.
- DO NOT edit any spec/instructions/*.yaml — those are owned by the oracle lane.
- DO NOT run git, commit, push, or create worktrees — the wrapper handles all git.
- Make these pass before you finish: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings; cargo build --workspace; cargo build --workspace --target wasm32-unknown-unknown."

omp -p --model "$IMPL_MODEL" --auto-approve --max-time "$IMPL_MAX_TIME" --cwd "$wt" "$prompt" \
  || { log "$id: omp run failed"; reopen "impl-lane: omp run failed/timed out"; exit 1; }

# tier-1 cheap gates — parallel across workers, no lock held
log "$id: tier-1 gates"
( cd "$wt" \
    && cargo fmt --all \
    && cargo clippy --workspace --all-targets -- -D warnings \
    && cargo build --workspace \
    && cargo build --workspace --target wasm32-unknown-unknown ) \
  || { log "$id: tier-1 failed"; reopen "impl-lane: tier-1 (fmt/clippy/build) failed"; exit 1; }

# exactly one commit on the worker branch
git -C "$wt" add -A
git -C "$wt" commit -m "$id: ${title:-implementation}" >/dev/null 2>&1 \
  || { log "$id: nothing to commit"; reopen "impl-lane: no changes produced"; exit 1; }

# tier-2 land under the merge slot
integrate "$wt" "$id"; rc=$?
case "$rc" in
  0) bd close "$id" >/dev/null 2>&1; log "$id: CLOSED" ;;
  1) reopen "impl-lane: tier-2 full test failed on integrated tree" ;;
  2) reopen "impl-lane: cherry-pick conflict vs $LANDING_BRANCH — redo against fresh tip" ;;
  *) reopen "impl-lane: integration error (slot)" ;;
esac
exit 0
