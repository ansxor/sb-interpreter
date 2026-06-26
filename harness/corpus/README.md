# corpus/ — committed test fixtures

| Path | What | Consumed by |
|---|---|---|
| `cases/*.yaml` | Cross-cutting `code`+`expect` cases (same shape as `spec/tests/`), not tied to one instruction. Deterministic + oracle-harvestable. | `diff/replay.py` (gate), `harvest.py` (fills expects) |
| `programs/*.sb3` | Full SmileBASIC programs. Some are self-checking (use `ASSERT__`); others are run on both sb-core and the oracle and compared. | `diff/replay.py`, `diff/run.py` |
| `golden/gfx/*.png` | Frozen oracle **GRP-page** framebuffers for pixel-diffing (M2+). Oracle-harvested via `run_case.py grp` (O-T6). | `diff/replay.py` |
| `golden/composite/*.png` | Frozen oracle **composite** framebuffers (sprites+BG+backdrop — the rendered display, not a GRP page). Oracle-harvested via `run_case.py composite` (O-T6 screenshot path). Oracle-truth storage only — no hermetic CI gate yet (`sb-run` can't render the full composited framebuffer; tracked in beads). | (spec `hw_verified` fills) |
| `golden/audio/*.wav` | Frozen oracle audio for sample/spectral-diffing (M5). | `diff/replay.py` |
| `sbsave/` | 3,329 real scraped programs + 2,773 resources (test **inputs**, not goldens). `INDEX.json` committed; unpacked tree regenerable via `tools/extract_sbsave.py`. | parser/e2e fuel; oracle-diff candidates. See `sbsave/README.md`. |

## Notes on the ported programs

- `programs/otya_test.sb3` — otya128's `osb` self-test (`SMILEBASIC/TEST.txt`), kept
  pristine. It uses the `ASSERT__` test-helper builtin (an `osb`/sb-core test-mode
  command, **not** real SB3), so it is a **sb-core-vs-osb golden**, not an oracle case:
  it encodes exact expected behavior the `osb` author verified (TinyMT RNG sequences,
  MAX/MIN 32-bit overflow wraparound, SORT/RSORT stability, string functions). Run it
  in M1 once sb-core implements `ASSERT__`.
- `programs/fizzbuzz.sb3` — a simple end-to-end demo program.

## Adding a case

Hand-author a `cases/*.yaml` entry, or let `harvest.py` promote a minimized fuzzer
finding. Either way, the expected output must come from real SB3 (documented values are
a starting hypothesis only — `hw_verified` is the bar).
