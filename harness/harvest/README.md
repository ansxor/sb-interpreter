# harvest/ — Phase A fixture generation (maintainer-run, never in PR CI)

`harvest.py` drives the emulator oracle + the fuzzer and writes the results back as
**committed** fixtures that the deterministic Phase-B gate replays:

- `spec/tests/<id>.yaml` — `expect:` blocks filled from real SmileBASIC; matching
  sources bumped to `confidence: hw_verified`.
- `harness/corpus/golden/gfx/*.png`, `.../audio/*.wav` — golden media.
- promoted, seeded fuzz regressions in `harness/corpus/`.

## Running (requires the emulator)

```bash
# Citra/Azahar running, SmileBASIC 3.6.0 loaded, scripting RPC enabled (UDP 45987)
python3 harness/harvest/harvest.py
git checkout -b harvest/refresh-$(date +%Y%m%d)
git add spec/tests harness/corpus/golden
# open a PR — CI then replays the refreshed fixtures deterministically
```

Keep this off the PR path. It is slow, stateful, and non-deterministic by nature; its
*output* is what makes the rest of the suite fast and reproducible.
