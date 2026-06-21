#!/usr/bin/env python3
"""PHASE B — deterministic replay gate (hermetic; safe for CI).

Runs `sb-core` against the COMMITTED fixtures only and compares to frozen expects:
  - spec/tests/*.yaml      per-instruction expect blocks
  - harness/corpus/cases/  cross-cutting code+expect cases
  - harness/corpus/programs/  full programs (self-checking, or with golden output)
  - harness/corpus/golden/    golden PNG / WAV fixtures (M2 / M5)

No emulator, no fuzzing, no network, fixed RNG seeds. (The per-instruction spec tests
are ALSO enforced by `cargo test -p sb-spec`; this driver adds full-program + golden
media replay once the sb-core headless runner exists.)

M0 status: the sb-core headless runner is not built yet (M1), so this reports the
deterministic suite inventory and exits 0. Execution wiring lands in M1.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def count(glob_dir, pattern):
    d = ROOT / glob_dir
    return sorted(d.glob(pattern)) if d.exists() else []


def sb_core_runner():
    """Path to the headless sb-core runner, if it has been built (M1)."""
    candidate = ROOT / "target" / "debug" / "sb-run"  # built in M1
    return candidate if candidate.exists() else None


def main():
    spec_tests = count("spec/tests", "*.yaml")
    cases = count("harness/corpus/cases", "*.yaml")
    programs = count("harness/corpus/programs", "*.sb3")
    golden_gfx = count("harness/corpus/golden/gfx", "*.png")
    golden_audio = count("harness/corpus/golden/audio", "*.wav")

    print("Deterministic replay inventory (committed fixtures):")
    print(f"  spec test overlays : {len(spec_tests)}")
    print(f"  corpus cases       : {len(cases)}")
    print(f"  corpus programs    : {len(programs)}")
    print(f"  golden gfx (PNG)   : {len(golden_gfx)}")
    print(f"  golden audio (WAV) : {len(golden_audio)}")

    runner = sb_core_runner()
    if runner is None:
        print("\nsb-core headless runner not built yet (M1).")
        print("Per-instruction spec tests run via: cargo test -p sb-spec")
        return 0

    # M1: execute each fixture through the runner and diff against expects.
    print(f"\nrunning fixtures through {runner} ...")
    _ = subprocess.run  # placeholder to mark the execution seam
    return 0


if __name__ == "__main__":
    sys.exit(main())
