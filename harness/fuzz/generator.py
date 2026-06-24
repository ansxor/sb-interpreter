#!/usr/bin/env python3
"""Seeded, spec-signature-driven SmileBASIC program generator (M7-T1).

Every program is generated from an explicit integer seed so any divergence the fuzzer
finds is exactly reproducible (`generate(seed)` -> the same program, byte-for-byte). When
the differential runner (`harness/diff/run.py`) finds a sb-core vs oracle mismatch — or a
sb-core crash — the offending seed's program is PROMOTED into `harness/corpus/fuzz/` as a
permanent, seeded regression that replays deterministically in CI (no emulator).

The grammar is driven by the typed signatures in `spec/instructions/*.yaml` (arg
counts/types + return types), so generated calls are well-typed enough to exercise real
behavior rather than just tripping a syntax error. Two profiles:

  * "safe"  — a runtime-safe, GUARANTEED-TERMINATING subset (math/string/bit functions +
              operators, bounded FOR loops only, no WHILE/GOTO/labels, no I/O or graphics).
              These can be run through the VM in a hermetic Rust test (`fuzz_corpus.rs`)
              because they always halt. This is the high-value numeric/string corner the
              M7 hardening targets (float-format, overflow, domain errors).
  * "broad" — additionally emits command (statement) forms across every category whose
              args are scalar-typed. May error or halt at runtime; used only for the
              parse+compile no-panic sweep, never run through the VM in CI.

Determinism is the whole point: the generator uses a single seeded `random.Random` and
NOTHING else (no time, no os-randomness), so a committed seed list reproduces byte-identical
programs on any machine.
"""
from __future__ import annotations

import argparse
import glob
import os
import random
from dataclasses import dataclass, field

try:
    import yaml  # PyYAML — only for READING the hand-authored specs.
except ImportError:  # pragma: no cover - yaml is a dev dependency (see harness/harvest)
    yaml = None

SPEC_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "spec", "instructions")

# --- type model -----------------------------------------------------------------
# Spec arg/return types are normalized to a small set the generator knows how to produce.
NUM_TYPES = {"integer", "number", "double", "int", "num"}
STR_TYPES = {"string", "str"}
# Types we cannot synthesize a literal/expression for in the simple grammar — a function or
# command taking one of these is skipped (arrays need a DIM'd var, labels/identifiers need
# program structure, varargs/keyword are special forms).
UNSUPPORTED_ARG_TYPES = {
    "array",
    "integer array",
    "number_array",
    "num_array",
    "any_array",
    "string_array",
    "identifier",
    "keyword",
    "label",
    "varargs",
}


def _norm(t: str | None) -> str:
    return (t or "").strip().lower()


# --- spec registry --------------------------------------------------------------
@dataclass
class Sig:
    arg_types: list[str]  # normalized arg types, in order
    ret: str  # normalized return type


@dataclass
class Instr:
    id: str
    kind: str  # "function" | "statement" | "operator"
    category: str
    sigs: list[Sig] = field(default_factory=list)


@dataclass
class Registry:
    instrs: list[Instr]

    # Functions usable inside a NUMERIC expression: at least one signature returns a number
    # and has only scalar (num/str/any) args.
    num_funcs: list[Instr] = field(default_factory=list)
    # Functions usable inside a STRING expression.
    str_funcs: list[Instr] = field(default_factory=list)
    # Commands (statements) with all-scalar args, for the "broad" profile.
    scalar_cmds: list[Instr] = field(default_factory=list)


def _scalar_only(sig: Sig) -> bool:
    return all(t not in UNSUPPORTED_ARG_TYPES for t in sig.arg_types)


def _ret_is_num(s: str) -> bool:
    return s in NUM_TYPES


def _ret_is_str(s: str) -> bool:
    return s in STR_TYPES


def load_registry(spec_dir: str = SPEC_DIR) -> Registry:
    """Parse every `spec/instructions/*.yaml` into a typed Registry (deterministic order)."""
    if yaml is None:
        raise RuntimeError("PyYAML is required to load spec signatures")
    instrs: list[Instr] = []
    for path in sorted(glob.glob(os.path.join(spec_dir, "*.yaml"))):
        spec = yaml.safe_load(open(path, encoding="utf-8"))
        if not isinstance(spec, dict):
            continue
        sigs: list[Sig] = []
        for raw in spec.get("signatures") or []:
            args = [_norm(a.get("type")) for a in (raw.get("args") or [])]
            ret = raw.get("returns") or {}
            ret_t = _norm(ret.get("type")) if isinstance(ret, dict) else ""
            sigs.append(Sig(arg_types=args, ret=ret_t))
        instrs.append(
            Instr(
                id=str(spec.get("id") or "").upper(),
                kind=_norm(spec.get("kind")),
                category=str(spec.get("category") or ""),
                sigs=sigs,
            )
        )
    reg = Registry(instrs=instrs)
    for ins in instrs:
        if not ins.id:
            continue
        if ins.kind == "function":
            if any(_scalar_only(s) and _ret_is_num(s.ret) for s in ins.sigs):
                reg.num_funcs.append(ins)
            if any(_scalar_only(s) and _ret_is_str(s.ret) for s in ins.sigs):
                reg.str_funcs.append(ins)
        elif ins.kind == "statement":
            if ins.sigs and any(_scalar_only(s) for s in ins.sigs):
                reg.scalar_cmds.append(ins)
    return reg


# A small, curated allowlist of categories/ids that are runtime-safe (no I/O, no graphics,
# no blocking input, no resource handles). The "safe" profile draws functions only from
# here so the generated program is guaranteed to terminate without side effects.
SAFE_CATEGORIES = {"Mathematics", "Strings", "Bit Operations"}
# A handful of math/string functions are intentionally excluded from the safe runtime set
# because they are non-deterministic (RNG) — they would still be terminating, but we keep
# the safe corpus reproducible across runs WITHOUT relying on RNDF's seed semantics.
SAFE_FUNC_DENY = {"RND", "RNDF"}


def _safe(funcs: list[Instr]) -> list[Instr]:
    return [
        f
        for f in funcs
        if f.category in SAFE_CATEGORIES and f.id not in SAFE_FUNC_DENY
    ]


# --- the generator --------------------------------------------------------------
class Gen:
    """One generation run, bound to a seeded RNG. Holds the declared-variable environment."""

    MAX_EXPR_DEPTH = 3
    NUM_VARS = ["A", "B", "C", "D", "E"]
    STR_VARS = ["A$", "B$", "C$", "D$"]

    def __init__(self, seed: int, registry: Registry, profile: str):
        self.rng = random.Random(seed)
        self.reg = registry
        self.profile = profile
        # Numeric vars that are CURRENTLY a FOR control variable in an enclosing loop. The
        # safe profile must never assign to one inside the loop body — reassigning the loop
        # counter makes a `FOR I=0 TO 0` run forever, which is a *legitimate* (the real SB
        # would loop too) but non-terminating program, defeating the "always halts" contract
        # the hermetic VM test relies on.
        self.locked_vars: set[str] = set()
        if profile == "safe":
            self.num_funcs = _safe(registry.num_funcs)
            self.str_funcs = _safe(registry.str_funcs)
        else:
            self.num_funcs = list(registry.num_funcs)
            self.str_funcs = list(registry.str_funcs)

    # -- expressions --
    def num_literal(self) -> str:
        r = self.rng
        choice = r.randint(0, 3)
        if choice == 0:
            return str(r.randint(-32, 32))
        if choice == 1:
            return str(r.randint(-1000000, 1000000))
        if choice == 2:
            # a double literal
            return f"{r.uniform(-100, 100):.4f}"
        # an interesting boundary value
        return r.choice(["0", "1", "-1", "2147483647", "-2147483648", "0.5", "3.14159"])

    def str_literal(self) -> str:
        # Keep to printable ASCII without quotes/newlines so the literal stays well-formed.
        alphabet = "ABCDEFGHIJ0123456789 "
        n = self.rng.randint(0, 6)
        return '"' + "".join(self.rng.choice(alphabet) for _ in range(n)) + '"'

    def num_expr(self, depth: int) -> str:
        r = self.rng
        if depth >= self.MAX_EXPR_DEPTH:
            return r.choice([self.num_literal, self.num_var])()
        roll = r.randint(0, 9)
        if roll <= 2:
            return self.num_literal()
        if roll == 3:
            return self.num_var()
        if roll == 4:
            # `-(...)` (never a bare `--literal`, which would lex oddly).
            return f"-({self.num_expr(depth + 1)})"
        if roll <= 7:
            op = r.choice(["+", "-", "*", "/", "DIV", "MOD", "AND", "OR", "<<", ">>"])
            a, b = self.num_expr(depth + 1), self.num_expr(depth + 1)
            return f"({a} {op} {b})"
        # a function call returning a number
        return self.num_call(depth) or self.num_literal()

    def str_expr(self, depth: int) -> str:
        r = self.rng
        if depth >= self.MAX_EXPR_DEPTH:
            return r.choice([self.str_literal, self.str_var])()
        roll = r.randint(0, 6)
        if roll <= 2:
            return self.str_literal()
        if roll == 3:
            return self.str_var()
        if roll == 4:
            return f"({self.str_expr(depth + 1)} + {self.str_expr(depth + 1)})"
        return self.str_call(depth) or self.str_literal()

    def any_expr(self, depth: int) -> str:
        return self.num_expr(depth) if self.rng.random() < 0.5 else self.str_expr(depth)

    def num_var(self) -> str:
        return self.rng.choice(self.NUM_VARS)

    def str_var(self) -> str:
        return self.rng.choice(self.STR_VARS)

    def _arg(self, t: str, depth: int) -> str:
        if t in STR_TYPES:
            return self.str_expr(depth + 1)
        if t == "any":
            return self.any_expr(depth + 1)
        # default everything else numeric (int/number/double)
        return self.num_expr(depth + 1)

    def _pick_sig(self, ins: Instr, want_num: bool) -> Sig | None:
        cands = [
            s
            for s in ins.sigs
            if _scalar_only(s)
            and (_ret_is_num(s.ret) if want_num else _ret_is_str(s.ret))
        ]
        return self.rng.choice(cands) if cands else None

    def num_call(self, depth: int) -> str | None:
        if not self.num_funcs:
            return None
        ins = self.rng.choice(self.num_funcs)
        sig = self._pick_sig(ins, want_num=True)
        if sig is None:
            return None
        args = ", ".join(self._arg(t, depth) for t in sig.arg_types)
        return f"{ins.id}({args})" if args else f"{ins.id}()"

    def str_call(self, depth: int) -> str | None:
        if not self.str_funcs:
            return None
        ins = self.rng.choice(self.str_funcs)
        sig = self._pick_sig(ins, want_num=False)
        if sig is None:
            return None
        args = ", ".join(self._arg(t, depth) for t in sig.arg_types)
        return f"{ins.id}({args})" if args else f"{ins.id}()"

    # -- statements --
    def assignable_num_var(self) -> str | None:
        """A numeric var that is safe to assign to (not an enclosing loop counter)."""
        free = [v for v in self.NUM_VARS if v not in self.locked_vars]
        return self.rng.choice(free) if free else None

    def stmt_assign(self) -> str:
        target = self.assignable_num_var()
        if target is not None and self.rng.random() < 0.7:
            return f"{target} = {self.num_expr(0)}"
        return f"{self.str_var()} = {self.str_expr(0)}"

    def stmt_print(self) -> str:
        if self.rng.random() < 0.6:
            return f"PRINT {self.num_expr(0)}"
        return f"PRINT {self.str_expr(0)}"

    def stmt_if(self, indent: int) -> list[str]:
        cond = f"{self.num_expr(0)} {self.rng.choice(['<', '>', '==', '!=', '<=', '>='])} {self.num_expr(0)}"
        return [f"IF {cond} THEN {self.simple_stmt()}"]

    def stmt_for(self, indent: int) -> list[str]:
        var = self.assignable_num_var()
        if var is None:  # all counters are taken — emit a plain statement instead
            return ["  " * indent + self.simple_stmt()]
        lo = self.rng.randint(0, 2)
        hi = lo + self.rng.randint(0, 3)  # bounded: at most 4 iterations
        self.locked_vars.add(var)
        body = []
        for _ in range(self.rng.randint(1, 2)):
            body.append("  " * (indent + 1) + self.simple_stmt())
        self.locked_vars.discard(var)
        lines = [f"{'  ' * indent}FOR {var} = {lo} TO {hi}"]
        lines += body
        lines.append(f"{'  ' * indent}NEXT")
        return lines

    def simple_stmt(self) -> str:
        """A single-line statement (no block) — for IF-THEN bodies and FOR bodies."""
        return self.rng.choice([self.stmt_assign, self.stmt_print])()

    def cmd_stmt(self) -> str | None:
        """A spec command statement with scalar args (broad profile only)."""
        cmds = self.reg.scalar_cmds
        if not cmds:
            return None
        ins = self.rng.choice(cmds)
        sig = self.rng.choice([s for s in ins.sigs if _scalar_only(s)])
        args = " ".join(
            (self._arg(t, 0) if i == 0 else ", " + self._arg(t, 0))
            for i, t in enumerate(sig.arg_types)
        )
        return f"{ins.id} {args}".rstrip()

    def block_stmt(self, indent: int) -> list[str]:
        r = self.rng
        roll = r.randint(0, 9)
        if roll <= 4:
            return ["  " * indent + self.stmt_assign()]
        if roll <= 6:
            return ["  " * indent + self.stmt_print()]
        if roll == 7:
            return ["  " * indent + s for s in self.stmt_if(indent)]
        if roll == 8 and indent < 1:  # bound nesting depth
            return self.stmt_for(indent)
        if self.profile != "safe":
            cmd = self.cmd_stmt()
            if cmd is not None:
                return ["  " * indent + cmd]
        return ["  " * indent + self.stmt_print()]

    def program(self, n_stmts: int) -> str:
        lines: list[str] = []
        # Pre-declare the variable pool so reads never hit an undefined variable.
        for v in self.NUM_VARS:
            lines.append(f"{v} = {self.num_literal()}")
        for v in self.STR_VARS:
            lines.append(f"{v} = {self.str_literal()}")
        for _ in range(n_stmts):
            lines.extend(self.block_stmt(0))
        return "\n".join(lines) + "\n"


# Cached registry so repeated generate() calls (a campaign) don't re-parse 248 files each time.
_REGISTRY: Registry | None = None


def _registry() -> Registry:
    global _REGISTRY
    if _REGISTRY is None:
        _REGISTRY = load_registry()
    return _REGISTRY


def generate(seed: int, profile: str = "safe", n_stmts: int = 12) -> str:
    """Return a SmileBASIC program string for the given seed (deterministic).

    `profile`: "safe" (terminating, runnable in CI) or "broad" (parse/compile sweep only).
    """
    return Gen(seed, _registry(), profile).program(n_stmts)


def main():
    ap = argparse.ArgumentParser(description="Generate a seeded SmileBASIC program.")
    ap.add_argument("seed", type=int)
    ap.add_argument("--profile", choices=["safe", "broad"], default="safe")
    ap.add_argument("--stmts", type=int, default=12)
    args = ap.parse_args()
    print(generate(args.seed, args.profile, args.stmts), end="")


if __name__ == "__main__":
    main()
