# shared config + helpers for the two-lane bead orchestrator.
# source this; do not execute. POSIX-bash-3.2 safe (no `wait -n`, no assoc arrays).

ORCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$ORCH_DIR" rev-parse --show-toplevel)"

# Everything the automation touches lives OUTSIDE your working tree, on a
# dedicated branch, so an unattended loop can never reset/clean your live edits.
WT_BASE="${SB_ORCH_WT_BASE:-$(dirname "$REPO_ROOT")/sb-orch}"
INTEG_WT="$WT_BASE/integration"        # the ONLY place lands happen
STATE_DIR="$WT_BASE/state"
LANDING_BRANCH="${SB_ORCH_BRANCH:-orch/landing}"

IMPL_MODEL="${IMPL_MODEL:-kimi}"        # omp --model (fuzzy match)
IMPL_CONCURRENCY="${IMPL_CONCURRENCY:-2}"
IMPL_MAX_TIME="${IMPL_MAX_TIME:-1800}"  # seconds, per-worker omp budget
ORACLE_LABEL="${ORACLE_LABEL:-oracle}"

mkdir -p "$STATE_DIR" 2>/dev/null || true

log(){ printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >&2; }

running_jobs(){ jobs -rp | grep -c . | tr -d ' '; }

# integrate <src-worktree> <bead-id>
# The caller has already made EXACTLY ONE commit on the src worktree's branch.
# Under the merge slot, cherry-pick that one commit onto the landing branch in
# the dedicated integration worktree, run the tier-2 full test, and either keep
# the commit (green) or discard the attempt (red/conflict).
# returns: 0 landed | 1 tier-2 test failed | 2 cherry-pick conflict | 3 slot error
integrate(){
  local src="$1" id="$2" commit rc=0
  commit="$(git -C "$src" rev-parse HEAD)" || return 3
  log "$id: waiting for merge slot…"
  bd merge-slot acquire --wait --holder "$id" >/dev/null 2>&1 || { log "$id: slot acquire failed"; return 3; }
  log "$id: slot held; cherry-pick $commit -> $LANDING_BRANCH"
  if ! git -C "$INTEG_WT" cherry-pick -n "$commit" >/dev/null 2>&1; then
    log "$id: CONFLICT vs $LANDING_BRANCH"; rc=2
  elif ! ( cd "$INTEG_WT" && cargo test --workspace ); then
    log "$id: TIER-2 full test FAILED on integrated tree"; rc=1
  else
    git -C "$INTEG_WT" commit -C "$commit" >/dev/null 2>&1 && log "$id: LANDED on $LANDING_BRANCH"
  fi
  if [ "$rc" != 0 ]; then
    # safe: INTEG_WT is dedicated + never hand-edited, so this only discards the
    # failed, uncommitted cherry-pick attempt — never any human work.
    git -C "$INTEG_WT" reset --hard HEAD >/dev/null 2>&1
    git -C "$INTEG_WT" clean -fd >/dev/null 2>&1
  fi
  bd merge-slot release --holder "$id" >/dev/null 2>&1 || true
  return $rc
}
