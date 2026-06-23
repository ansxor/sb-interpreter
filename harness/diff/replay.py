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
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
from png_util import decode_rgba, diff_rgba  # noqa: E402


def count(glob_dir, pattern):
    d = ROOT / glob_dir
    return sorted(d.glob(pattern)) if d.exists() else []


def sb_core_runner():
    """Path to the headless sb-core runner, if it has been built (M1)."""
    candidate = ROOT / "target" / "debug" / "sb-run"  # built in M1
    return candidate if candidate.exists() else None


def diff_goldens(runner):
    """M2-T5 graphics gate: render each golden program through `sb-run --grp` and pixel-diff
    its GRP display page against the committed golden PNG. Hermetic — no emulator: the golden
    is a frozen fixture (an oracle GRP capture, O-T6; or an oracle-pending sb-core render).
    Returns the list of FAILED golden stems (a pixel mismatch or a runner failure)."""
    goldens = count("harness/corpus/golden/gfx", "*.png")
    if not goldens:
        return []
    print(f"\npixel-diffing {len(goldens)} graphics golden(s) via {runner.name} ...")
    failed = []
    for golden in goldens:
        prog = golden.with_suffix(".sb3")
        if not prog.exists():
            print(f"  [FAIL] {golden.name}: no sibling program {prog.name}")
            failed.append(golden.stem)
            continue
        with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tf:
            tmp = Path(tf.name)
        try:
            proc = subprocess.run(
                [str(runner), "--grp", str(tmp), str(prog)],
                capture_output=True,
                text=True,
            )
            if proc.returncode not in (0, 1):  # 1 = SB error, page still rendered
                note = proc.stderr.strip().splitlines()[0] if proc.stderr.strip() else ""
                print(f"  [FAIL] {golden.name}: sb-run exit {proc.returncode}  {note}")
                failed.append(golden.stem)
                continue
            bad, total, first = diff_rgba(
                decode_rgba(golden.read_bytes()), decode_rgba(tmp.read_bytes())
            )
            if bad == 0:
                print(f"  [PASS] {golden.name}  ({total} px)")
            else:
                where = "size mismatch" if first is None else f"first @ {first}"
                print(f"  [FAIL] {golden.name}: {bad}/{total} px differ ({where})")
                failed.append(golden.stem)
        finally:
            tmp.unlink(missing_ok=True)
    return failed


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
    # `EXPECTED_PASS` programs gate the exit code; everything else is informational. The
    # FULL `otya_test.sb3` is still informational — it exercises sprite CALL, DTREAD,
    # DATE$/TIME$ and out-of-range decimal-literal i32 wrap (later milestones) — but its
    # M1-implemented subset is gated as `otya_m1.sb3` (the real golden's RANDOMTEST/MAXMIN/
    # SORT/SWAP/REPEAT/IF/math/string/DATA assertions; see PRD.md M1-T14).
    expected_pass = {"m1_conformance.sb3", "fizzbuzz.sb3", "otya_m1.sb3"}
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

    golden_failed = diff_goldens(runner)

    if failed or golden_failed:
        if failed:
            print(f"\n{len(failed)} expected-pass program(s) failed: {', '.join(failed)}")
        if golden_failed:
            print(f"{len(golden_failed)} graphics golden(s) failed: {', '.join(golden_failed)}")
        return 1
    print("\nall expected-pass programs OK; all graphics goldens match.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
