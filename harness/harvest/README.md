# harvest/ — Phase A fixture generation (maintainer-run, never in PR CI)

`harvest.py` drives the real SmileBASIC 3.6.0 oracle (Azahar, via the `sb-oracle` skill's
`run_case.py`) and writes the results back as **committed** fixtures that the deterministic
Phase-B gate replays without any emulator:

- `spec/tests/<stem>.yaml` — machine-owned overlays carrying `expect:` values harvested from
  real SB (`hw_verified`). The sb-spec loader merges them onto the hand-authored inline specs,
  so the spec files themselves are never rewritten.
- `harness/corpus/golden/gfx/*.png`, `.../audio/*.wav` — golden media (captured separately by
  `run_case.py grp` / `screenshot`; see O-T6/O-T7).

## What it does (`harvest.py <stems...>`)

1. **collect** every inline `tests:` case from `spec/instructions/<stem>.yaml` and turn each into
   an oracle batch case-line (`name|expr`, `name|expr|str`, or `name|stmt|err`), picking the mode
   from the test's code + expect + the signature return type.
2. **capture** by calling `run_case.py batch CASEFILE OUTFILE` (one mega-program for value cases;
   error cases run alone). The OUTFILE is incremental + resumable — a killed run keeps its
   partials and a re-run only retries failures.
3. **fold**: parse the TSV, diff each captured result against the inline expect
   (CONFIRMED / MISMATCH / NEW / failed-capture), and write the harvested values into the
   `spec/tests/<stem>.yaml` overlay.
4. **report** the summary + the manual `git`/PR steps. A `confidence: hw_verified` bump on the
   spec source stays a reviewed manual edit (we don't reflow hand-authored YAML).

## Running (requires the emulator)

```bash
# Azahar running, SmileBASIC 3.6.0 on the DIRECT-mode screen, OBOOT on the SMILE button.
python3 harness/harvest/harvest.py abs floor mid       # specific stems
python3 harness/harvest/harvest.py --category Mathematics
python3 harness/harvest/harvest.py --all               # the whole spec

git checkout -b harvest/refresh-$(date +%Y%m%d)
git add spec/tests && git commit
# open a PR — CI then replays the refreshed fixtures deterministically
```

### Offline (no emulator)

```bash
python3 harness/harvest/harvest.py abs --from-tsv out/abs.tsv   # fold an existing capture
python3 harness/harvest/test_harvest.py                         # unit-test the pure logic
```

Keep this off the PR path. It is slow, stateful, and non-deterministic by nature; its *output*
is what makes the rest of the suite fast and reproducible.
