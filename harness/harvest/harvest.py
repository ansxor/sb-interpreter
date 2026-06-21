#!/usr/bin/env python3
"""PHASE A — harvest committed fixtures from the oracle + fuzzer (offline only).

Pipeline:
  1. For every spec test case (spec/tests/*.yaml) and corpus case, run it on the
     oracle (harness/oracle/extdata) and record the real stdout/error/value.
  2. Write the captured result back into the spec/tests overlay's `expect:` block and
     bump the matching source to confidence: hw_verified.
  3. For graphics/audio cases, save golden PNG/WAV under harness/corpus/golden/.
  4. Run the fuzzer (seeded), diff vs oracle, minimize divergences, and promote them
     into harness/corpus/ as permanent seeded regression cases.
  5. Open a PR with the refreshed fixtures.

This is the ONLY place that talks to the emulator for ground truth. It must never run
in PR CI — it's slow, stateful, and needs the emulator. Run it by hand or on a
schedule. The fixtures it produces are what the deterministic Phase-B gate replays.

Overlay YAML is emitted via json.dumps (a subset of YAML) so no pyyaml dependency is
needed and output is deterministic.

M0: scaffold. Real capture depends on the oracle spikes (harness/oracle/extdata.py).
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC_TESTS = ROOT / "spec" / "tests"


def yscalar(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    return json.dumps(v, ensure_ascii=False)


def write_overlay(stem: str, cases: list) -> Path:
    """Deterministically (re)write spec/tests/<stem>.yaml from harvested cases.

    `cases` is a list of dicts: {name, code, expect:{stdout?|error:{errnum}?}}.
    """
    lines = [f"# Oracle-harvested fixtures for {stem.upper()} — do not edit by hand.", "tests:"]
    for c in cases:
        lines.append(f"  - name: {yscalar(c['name'])}")
        lines.append(f"    code: {yscalar(c['code'])}")
        exp = c.get("expect", {})
        if "error" in exp:
            lines.append(f"    expect: {{ error: {{ errnum: {int(exp['error']['errnum'])} }} }}")
        else:
            lines.append(f"    expect: {{ stdout: {yscalar(exp.get('stdout', ''))} }}")
    SPEC_TESTS.mkdir(parents=True, exist_ok=True)
    path = SPEC_TESTS / f"{stem}.yaml"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def main():
    print("harvest (Phase A) — pending oracle spikes; no emulator contact in M0.")
    print(f"would write overlays into: {SPEC_TESTS}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
