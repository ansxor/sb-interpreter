#!/usr/bin/env python3
"""PHASE A — 3-way differential runner (offline; needs the emulator).

For each corpus item, run it through:
  (a) sb-core            our interpreter
  (b) osb                otya128's D interpreter (3.5.0, optional cross-check)
  (c) the oracle         real SmileBASIC 3.6.0 in Citra/Azahar

and report disagreements. Where (a) != (c), that's a bug in ours; where (b) != (c),
that's a known osb divergence (osb targets 3.5.0). Confirmed (c) results feed
`harvest.py`, which freezes them into committed fixtures for Phase B.

Stub: wiring depends on the oracle spikes (see harness/oracle/extdata.py) and the
sb-core headless runner (M1).
"""
import sys


def main():
    print("differential runner (Phase A) — pending oracle spikes + sb-core M1 runner")
    return 0


if __name__ == "__main__":
    sys.exit(main())
