#!/usr/bin/env bash
# One-time setup for the two-lane orchestrator. Idempotent; safe to re-run.
set -u
source "$(dirname "$0")/lib.sh"

# 1. merge slot (the single integration mutex shared by both lanes)
bd merge-slot create >/dev/null 2>&1 && log "created merge slot" || log "merge slot already exists"

# 2. dedicated landing branch, seeded from main (never hand-edited)
if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$LANDING_BRANCH"; then
  log "landing branch $LANDING_BRANCH exists"
else
  git -C "$REPO_ROOT" branch "$LANDING_BRANCH" main && log "created $LANDING_BRANCH from main"
fi

# 3. dedicated integration worktree (the only place lands happen)
if [ -d "$INTEG_WT" ]; then
  log "integration worktree exists: $INTEG_WT"
else
  git -C "$REPO_ROOT" worktree add "$INTEG_WT" "$LANDING_BRANCH" && log "integration worktree: $INTEG_WT"
fi

cat >&2 <<EOF

setup complete.
  worktree base : $WT_BASE   (outside the repo, gitignored by location)
  landing branch: $LANDING_BRANCH
  merge slot    : $(bd merge-slot check 2>&1 | head -1)

next:
  - label your oracle/harvest beads:   bd label add <id> $ORACLE_LABEL
  - run the impl lane:                 harness/orchestration/impl-dispatch.sh
  - run the oracle lane:               harness/orchestration/oracle-lane.sh
  - stop the oracle lane:              touch $STATE_DIR/oracle/STOP
  - promote landed work to main (you): git checkout main && git merge $LANDING_BRANCH
EOF
