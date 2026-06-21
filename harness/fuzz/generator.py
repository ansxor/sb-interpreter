#!/usr/bin/env python3
"""Seeded, grammar-aware SmileBASIC program generator (Phase A).

Every program is generated from an explicit integer seed so any divergence the fuzzer
finds is exactly reproducible (`generate(seed)` -> the same program). When the
differential runner finds a sb-core vs oracle mismatch, the offending program is
minimized and PROMOTED into harness/corpus/ as a permanent, seeded regression test —
so the fuzzer never has to rediscover it.

The grammar draws on the spec signatures in `spec/instructions/` (arg counts/types) so
generated calls are well-typed enough to exercise real behavior rather than just
syntax errors.

M0: scaffold + seed plumbing. The grammar lands alongside the M1 parser/VM.
"""
import argparse
import random


def generate(seed: int) -> str:
    """Return a SmileBASIC program string for the given seed (deterministic)."""
    rng = random.Random(seed)
    # Placeholder: emit a trivial, deterministic program. The real grammar (spec-driven
    # expression/statement generation) is implemented with the M1 language core.
    n = rng.randint(0, 9)
    return f"PRINT {n}\n"


def main():
    ap = argparse.ArgumentParser(description="Generate a seeded SmileBASIC program.")
    ap.add_argument("seed", type=int)
    args = ap.parse_args()
    print(generate(args.seed), end="")


if __name__ == "__main__":
    main()
