#!/usr/bin/env bash
# Land one already-committed worktree onto the landing branch under the merge
# slot, gated by the tier-2 full test. Callable by either lane.
#   integrate.sh <src-worktree> <bead-id>
# exit: 0 landed | 1 tier-2 failed | 2 conflict | 3 slot/error
set -u
source "$(dirname "$0")/lib.sh"
integrate "${1:?usage: integrate.sh <src-worktree> <bead-id>}" "${2:?bead-id required}"
