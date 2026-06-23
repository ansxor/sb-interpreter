#!/usr/bin/env bash
set -u

# Watch for Ralph done/stop sentinel files and kill only the current child
# process tree under a long-running parent/wrapper process.
#
# Usage:
#   ./ralph-continuous-watch.sh <parent-pid>
#
# Example:
#   ./ralph-continuous-watch.sh 72095
#
# You can also override the watched directory:
#   WATCH_DIR=/path/to/repo ./ralph-continuous-watch.sh <parent-pid>

watch_dir="${WATCH_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
parent="${1:-${PARENT_PID:-}}"

if [ -z "$parent" ]; then
  echo "usage: $0 <parent-pid>" >&2
  echo "or:    PARENT_PID=<parent-pid> $0" >&2
  exit 2
fi

if ! [[ "$parent" =~ ^[0-9]+$ ]]; then
  echo "error: parent pid must be numeric, got: $parent" >&2
  exit 2
fi

cd "$watch_dir" || exit 1

sentinel_seen() {
  [ -f .ralph/DONE ] ||
  [ -f .ralph/done ] ||
  [ -f .ralph/STOP ] ||
  [ -f Ralph.done ] ||
  [ -f ralph.done ]
}

kill_tree() {
  local pid="$1" child

  for child in $(pgrep -P "$pid" 2>/dev/null); do
    kill_tree "$child"
  done

  kill -TERM "$pid" 2>/dev/null || true
}

kill_tree_hard() {
  local pid="$1" child

  for child in $(pgrep -P "$pid" 2>/dev/null); do
    kill_tree_hard "$child"
  done

  kill -KILL "$pid" 2>/dev/null || true
}

last_hit=0

echo "$(date '+%F %T') watching $watch_dir for Ralph sentinels; parent=$parent"

while kill -0 "$parent" 2>/dev/null; do
  if sentinel_seen; then
    now=$(date +%s)

    # Avoid double-killing the same sentinel while the wrapper is between iterations.
    if [ $((now - last_hit)) -ge 5 ]; then
      for p in $(pgrep -P "$parent" 2>/dev/null); do
        kill_tree "$p"
      done

      sleep 2

      for p in $(pgrep -P "$parent" 2>/dev/null); do
        kill_tree_hard "$p"
      done

      echo "$(date '+%F %T') continuous watcher saw Ralph sentinel and killed child tree(s) for parent $parent"
      last_hit=$now
    fi
  fi

  sleep 2
done

echo "$(date '+%F %T') parent $parent exited; watcher stopping"
