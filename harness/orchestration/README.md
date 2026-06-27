# Two-lane bead orchestrator

Decouples the **serial oracle** from **parallel implementation** so the single
Azahar emulator stops gating all development.

```
ORACLE LANE   serial, one process      label:oracle    drives Azahar, owns the spec
IMPL LANE     parallel, N workers       (everything else) writes sb-core code + tests
```

The spec yaml is the contract and the hand-off: an impl bead `blocked-by` an open
oracle bead is auto-hidden from the impl lane until the harvest lands.

## Model

- **Oracle lane** (`oracle-lane.sh`): a Ralph-style self-restarting loop (one
  `claude` at a time). Serial *by construction* — correct, because there is one
  emulator. Picks one `oracle`-labelled bead, harvests via the sb-oracle skill,
  finalizes the spec, commits, lands. Restarts fresh per bead (`DONE`) so context
  never bloats. If Azahar is down it parks (`STOP`) instead of stalling the impl
  lane — a human relaunches Azahar and re-runs the script.
- **Impl lane** (`impl-dispatch.sh` → `impl-worker.sh`): one dispatcher (sole
  claimer → no race) caps concurrency at `IMPL_CONCURRENCY` (default 2) and spawns
  one worker per bead. Each worker runs `omp -p --model "$IMPL_MODEL"` in its own
  git worktree, runs the **tier-1** cheap gates, then lands under the merge slot.

## Landing tests (two tiers)

| tier | where | what | parallel? |
|------|-------|------|-----------|
| 1 | in the worker's worktree | `fmt` · `clippy` · `build` · `build --target wasm32` | yes |
| 2 | in the integration worktree, under the merge slot | `cargo test --workspace` | no (runs once per land) |

The expensive full suite (~5 min) runs **once at the chokepoint**, never N times
in parallel. Cheap checks fail fast without holding the lock.

## Merge strategy — walled off from your working tree

The automation **never touches `main` or your working tree.** It lands on a
dedicated branch `orch/landing` inside dedicated worktrees under `../sb-orch/`:

```
land(commit):  hold merge slot, in the integration worktree:
   git cherry-pick -n <worker-commit>     # one task -> one commit; no rebase, no rewrite, no push
   cargo test --workspace                 # tier-2 gate
   green     -> git commit -C <commit>     (lands on orch/landing)
   red/conflict -> git reset --hard + clean   (safe: dedicated, never-edited tree) -> reopen bead
```

`cherry-pick` (not rebase/merge) keeps history honest per `CLAUDE.md §5`: one new
commit per task, shared history never rewritten. Conflicts are rare (impl beads
touch disjoint code and may not edit the instruction yaml — that's the oracle
lane's file); on conflict the bead is reopened to be redone against fresh tip.

**You** promote landed work when ready:

```
git checkout main && git merge orch/landing
```

### Destructive ops, scoped on purpose

`git reset --hard` / `git clean -fd` run **only** inside `../sb-orch/integration`,
a dedicated checkout nobody hand-edits, and **only** to discard a failed,
uncommitted cherry-pick. They never run in your repo root, so your uncommitted
edits are never at risk. `git branch -D` only deletes throwaway `worker-*`
branches whose commit was already cherry-picked or discarded.

## Usage

```bash
# one-time
harness/orchestration/setup.sh
bd label add <id> oracle        # tag each harvest bead (migration is manual)

# run (separate terminals)
harness/orchestration/impl-dispatch.sh
harness/orchestration/oracle-lane.sh

# stop the oracle lane
touch ../sb-orch/state/oracle/STOP
```

### Env knobs

| var | default | meaning |
|-----|---------|---------|
| `IMPL_CONCURRENCY` | `2` | parallel impl workers |
| `IMPL_MODEL` | `kimi` | `omp --model` (fuzzy) |
| `IMPL_MAX_TIME` | `1800` | per-worker omp seconds |
| `SB_ORCH_WT_BASE` | `../sb-orch` | worktree base (outside repo) |
| `SB_ORCH_BRANCH` | `orch/landing` | landing branch |
| `ORACLE_LABEL` | `oracle` | label marking the serial lane |

## Prerequisites

- `bd` (beads), `jq`, `cargo`, the `wasm32-unknown-unknown` target
- `omp` on PATH (impl brain); `claude` on PATH (oracle brain, needs the sb-oracle skill)
- Azahar running with SB 3.6.0 in DIRECT mode for the oracle lane
