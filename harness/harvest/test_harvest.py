#!/usr/bin/env python3
"""Offline tests for harvest.py — the COLLECT/PARSE/FOLD logic that must work without Azahar.

Run: `python3 harness/harvest/test_harvest.py` (no emulator, no network, deterministic).
These guard the pure transforms so the Phase-A driver can be trusted before it ever touches
the oracle. The oracle-driving part (run_oracle) is exercised by hand against real SB.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harvest as H  # noqa: E402


def eq(got, want, msg):
    assert got == want, f"{msg}: got {got!r}, want {want!r}"


def test_returns_string():
    eq(H.returns_string({"id": "MID$"}), True, "id ending in $ is a string")
    eq(H.returns_string({"id": "ABS"}), False, "ABS is numeric")
    eq(H.returns_string({"id": "X", "signatures": [{"returns": {"type": "string"}}]}), True,
       "string return type")
    eq(H.returns_string({"id": "X", "signatures": [{"returns": {"type": "number"}}]}), False,
       "number return type")


def test_test_to_caseline():
    abs_spec = {"id": "ABS", "signatures": [{"returns": {"type": "number"}}]}
    mid_spec = {"id": "MID$", "signatures": [{"returns": {"type": "string"}}]}

    name, line, mode, reason = H.test_to_caseline(
        abs_spec, {"name": "neg", "code": "PRINT ABS(-5)", "expect": {"stdout": "5"}})
    eq((line, mode, reason), ("abs.neg|ABS(-5)", "num", None), "numeric value case")

    name, line, mode, _ = H.test_to_caseline(
        mid_spec, {"name": "basic", "code": 'PRINT MID$("ABC",0,2)', "expect": {"stdout": "AB"}})
    eq((line, mode), ('mid.basic|MID$("ABC",0,2)|str', "str"), "string value case (|str)")

    name, line, mode, _ = H.test_to_caseline(
        abs_spec, {"name": "err", "code": 'A=ABS("x")', "expect": {"error": {"errnum": 8}}})
    eq((line, mode), ('abs.err|A=ABS("x")|err', "err"), "error case (|err)")

    # Non-PRINT, non-error code can't be batch-harvested as a value -> skipped with a reason.
    name, line, mode, reason = H.test_to_caseline(
        abs_spec, {"name": "weird", "code": "A=5:PRINT A", "expect": {"stdout": "5"}})
    eq(line, None, "multi-statement is skipped")
    assert reason, "skip carries a reason"


def test_parse_tsv():
    # Mirrors the real run_case batch output: value lines, an errnum line, a resumed (cached)
    # line, and a `#` progress note — all from the committed st5c fixture shape.
    text = (
        "# resume: 3 case(s) already harvested, skipping them\n"
        "chkchr_read\t65\n"
        "width_query_default\t8\t(cached)\n"
        "attr_16\terrnum=10 errline=1\n"
        "\n"
    )
    got = H.parse_tsv(text)
    eq(got, {"chkchr_read": "65", "width_query_default": "8", "attr_16": "errnum=10 errline=1"},
       "parse_tsv handles value/cached/errnum/comment/blank")


def test_parse_tsv_committed_fixture():
    fixture = Path(__file__).resolve().parent / "out" / "st5c.tsv"
    if not fixture.exists():
        return  # fixture optional
    got = H.parse_tsv(fixture.read_text(encoding="utf-8"))
    eq(got["chkchr_read"], "65", "committed fixture value")
    eq(got["attr_16"], "errnum=10 errline=1", "committed fixture error")


def test_raw_to_expect():
    eq(H.raw_to_expect("12.345"), {"stdout": "12.345"}, "value -> stdout")
    eq(H.raw_to_expect("errnum=10 errline=1"), {"error": {"errnum": 10}}, "errnum -> error")
    eq(H.raw_to_expect("ERROR prog case halted"), None, "harvest failure -> None")
    eq(H.raw_to_expect("NOERR (statement did not raise)"), None, "no-raise -> None")


def test_expects_equal():
    assert H.expects_equal({"stdout": "5"}, {"stdout": "5"})
    assert not H.expects_equal({"stdout": "5"}, {"stdout": "6"})
    assert H.expects_equal({"error": {"errnum": 8}}, {"error": {"errnum": 8}})
    assert not H.expects_equal({"error": {"errnum": 8}}, {"error": {"errnum": 10}})
    assert not H.expects_equal({"stdout": "5"}, {"error": {"errnum": 8}})


def test_render_overlay_deterministic():
    cases = [
        {"name": "neg", "code": "PRINT ABS(-5)", "expect": {"stdout": "5"}},
        {"name": "err", "code": 'A=ABS("x")', "expect": {"error": {"errnum": 8}}},
    ]
    a = H.render_overlay("abs", cases)
    b = H.render_overlay("abs", cases)
    eq(a, b, "render is deterministic")
    assert "tests:" in a and "stdout: \"5\"" in a and "errnum: 8" in a, "overlay shape"
    # Overlay must parse back into the loader's TestOverlay shape.
    import yaml
    parsed = yaml.safe_load(a)
    eq(len(parsed["tests"]), 2, "two tests round-trip")
    eq(parsed["tests"][0]["expect"]["stdout"], "5", "stdout round-trips")
    eq(parsed["tests"][1]["expect"]["error"]["errnum"], 8, "errnum round-trips")


def test_fold_end_to_end():
    # A synthetic spec with one confirmed, one mismatch, one new, one failed case.
    spec = {"id": "ABS"}
    cases = [
        {"stem": "abs", "name": "abs.ok", "test": {"name": "ok", "code": "PRINT ABS(-5)",
         "expect": {"stdout": "5"}}},
        {"stem": "abs", "name": "abs.bad", "test": {"name": "bad", "code": "PRINT ABS(-3.5)",
         "expect": {"stdout": "3"}}},  # inline wrong on purpose
        {"stem": "abs", "name": "abs.fresh", "test": {"name": "fresh", "code": "PRINT ABS(0)"}},
        {"stem": "abs", "name": "abs.broke", "test": {"name": "broke", "code": "PRINT ABS(1)",
         "expect": {"stdout": "1"}}},
    ]
    results = {"abs.ok": "5", "abs.bad": "3.5", "abs.fresh": "0",
               "abs.broke": "ERROR prog case halted"}
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        H.SPEC_TESTS = Path(d)  # redirect overlay output so the test never writes into spec/
        report = H.fold(cases, results, {"abs": spec})
    eq(report["confirmed"], ["abs.ok"], "ok confirmed")
    eq([m[0] for m in report["mismatch"]], ["abs.bad"], "bad flagged as mismatch")
    eq(report["new"], ["abs.fresh"], "fresh is new")
    eq([f[0] for f in report["failed"]], ["abs.broke"], "broke is a capture failure")
    eq(len(report["written"]), 1, "one overlay written")


def main():
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for t in tests:
        t()
        print(f"  ok  {t.__name__}")
    print(f"\n{len(tests)} harvest tests passed.")


if __name__ == "__main__":
    main()
