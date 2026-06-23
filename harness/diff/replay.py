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
        print("\nsb-core headless runner not built yet — build it with:")
        print("  cargo build -p sb-platform-native --bin sb-run")
        print("Per-instruction + curated cases run via: cargo test -p sb-core")
        return 0

    # M1-T14: execute each self-checking full program through the runner. A program that
    # uses `ASSERT__` exits non-zero on the first failed assertion (the VM halts), so exit
    # code 0 means every assertion held. The per-case code→expect YAML fixtures
    # (harness/corpus/cases + spec/tests + in-scope spec inline tests) are replayed by the
    # hermetic `cargo test -p sb-core` conformance runner; here we add full-program replay.
    #
    # `EXPECTED_PASS` programs gate the exit code; everything else is informational (e.g.
    # `otya_test.sb3` exercises SORT/RSORT, sprite CALL, DTREAD, DATE$/TIME$ and
    # out-of-range decimal-literal i32 wrap — features that land in later milestones).
    expected_pass = {"m1_conformance.sb3", "fizzbuzz.sb3"}
    print(f"\nrunning {len(programs)} full program(s) through {runner.name} ...")
    failed = []
    for prog in programs:
        proc = subprocess.run(
            [str(runner), str(prog)], capture_output=True, text=True
        )
        gated = prog.name in expected_pass
        ok = proc.returncode == 0
        tag = "PASS" if ok else "FAIL"
        if not gated and not ok:
            tag = "skip"  # informational: blocked on a later milestone
        note = ""
        if not ok and proc.stderr.strip():
            note = "  " + proc.stderr.strip().splitlines()[0]
        print(f"  [{tag}] {prog.name}{note}")
        if gated and not ok:
            failed.append(prog.name)

    if failed:
        print(f"\n{len(failed)} expected-pass program(s) failed: {', '.join(failed)}")
        return 1
    print("\nall expected-pass programs OK.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
