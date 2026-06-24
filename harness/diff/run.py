#!/usr/bin/env python3
"""M7-T1 — the differential fuzzing runner.

Drives `harness/fuzz/generator.py` over a seed range and runs each generated program
through `sb-core` (the headless `sb-run` binary). It classifies every outcome and — the
whole point — flags any program that makes `sb-core` **panic** (a host crash SmileBASIC
itself never does: an integer overflow, a non-char-boundary slice, an unwrap on None …).
Those are bugs; minimize the seed to a small program and PROMOTE it into
`harness/corpus/fuzz/` as a permanent, seeded regression (then it replays in CI via
`crates/sb-core/tests/fuzz_corpus.rs` — no emulator needed).

Outcome classes (by `sb-run` exit code):
  ok        (0)  ran to completion / END / STOP
  sb_error  (1)  a normal SmileBASIC error (ERRNUM/ERRLINE on stderr) — expected, not a bug
  host_err  (2)  usage/host error (unreadable file) — should not happen here
  CRASH   (else)  a Rust panic — A BUG. Stderr is printed.
  timeout        ran past the wall-clock limit (a legitimately non-terminating program in the
                 broad profile, e.g. `VSYNC <huge>`; in the safe profile it would be a bug)

The THREE-way differential vs the real-hardware oracle (sb-core vs osb vs SmileBASIC 3.6.0
in Azahar) is the offline maintainer step — it needs the emulator, so like O-T8's live
harvest it is NOT run in the hermetic loop. `--oracle` is reserved for that wiring; today the
in-loop campaign is the deterministic sb-core robustness sweep, which already surfaces the
crash class the float/overflow hardening targets.

Usage:
  python3 harness/diff/run.py --profile safe  --seeds 1000
  python3 harness/diff/run.py --profile broad --seeds 1000 --timeout 5
"""
from __future__ import annotations

import argparse
import collections
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from harness.fuzz.generator import generate  # noqa: E402

SB_RUN = os.environ.get("SB_RUN", "target/debug/sb-run")


def run_program(src: str, timeout: float) -> tuple[object, str]:
    """Run one program through sb-run; return (returncode|'TIMEOUT', stderr)."""
    with tempfile.NamedTemporaryFile("w", suffix=".sb3", delete=False) as f:
        f.write(src)
        path = f.name
    try:
        p = subprocess.run([SB_RUN, path], capture_output=True, timeout=timeout)
        return p.returncode, p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", ""
    finally:
        os.unlink(path)


def classify(rc: object) -> str:
    return {0: "ok", 1: "sb_error", 2: "host_err"}.get(rc, "CRASH" if rc != "TIMEOUT" else "timeout")


def campaign(profile: str, seeds: range, timeout: float) -> int:
    if not os.path.exists(SB_RUN):
        print(f"error: {SB_RUN} not found — build it first: cargo build --bin sb-run", file=sys.stderr)
        return 2
    counts: collections.Counter[str] = collections.Counter()
    crashes: list[tuple[int, object, str]] = []
    for seed in seeds:
        src = generate(seed, profile=profile)
        rc, err = run_program(src, timeout)
        kind = classify(rc)
        counts[kind] += 1
        if kind == "CRASH":
            crashes.append((seed, rc, err))
    print(f"=== {profile} profile, {len(seeds)} seeds ===")
    print("  " + ", ".join(f"{k}={counts[k]}" for k in sorted(counts)))
    for seed, rc, err in crashes:
        print(f"\n!!! CRASH seed={seed} rc={rc} (promote: python3 harness/fuzz/generator.py {seed} --profile {profile})")
        print("    " + err.strip().replace("\n", "\n    "))
    if crashes:
        print(f"\n{len(crashes)} crash(es) — minimize + promote into harness/corpus/fuzz/.")
    return 1 if crashes else 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Differential fuzzing runner (M7-T1).")
    ap.add_argument("--profile", choices=["safe", "broad"], default="safe")
    ap.add_argument("--seeds", type=int, default=1000, help="run seeds [0, N)")
    ap.add_argument("--start", type=int, default=0)
    ap.add_argument("--timeout", type=float, default=5.0)
    args = ap.parse_args()
    return campaign(args.profile, range(args.start, args.start + args.seeds), args.timeout)


if __name__ == "__main__":
    sys.exit(main())
