# sbsave/ — real-world program corpus (scraped public server)

3,329 real SmileBASIC programs + 2,773 data/graphics resources, scraped from
**smilebasicsource.com**'s public server before it shut down. Use as test cases:
parser fuel, end-to-end runs, and (once the oracle is up) differential goldens.

This is the **decoded** view of `sbsave/sucess/` (the raw PETC `body.bin` blobs).
Decoder + format spec live in [`tools/extract_sbsave.py`](../../../tools/extract_sbsave.py).

## What's committed vs regenerated

| Path | Committed? | What |
|---|---|---|
| `INDEX.json` | ✅ yes (786 KB) | Manifest: every download → its files, with author, date, refcount, byte size, encoding. **The reference.** |
| `files/<KEY>/{TXT,DAT}/<NAME>` | ❌ no (gitignored, ~765 MB) | The unpacked tree. Regenerate: `python3 tools/extract_sbsave.py`. |

`<KEY>` is the server's file key (e.g. `12C3NWQE`). `TXT/` holds program source
(UTF-8 SmileBASIC), `DAT/` holds resources (`PCBN0001` graphics / numeric arrays).
The layout mirrors SmileBASIC's on-device `PROJECTS/<name>/{TXT,DAT}/`, so it drops
straight into the extdata injector (`harness/oracle/extdata.py`, task O-T3).

## Using it

```bash
python3 tools/extract_sbsave.py              # unpack all + (re)write INDEX.json
python3 tools/extract_sbsave.py --manifest-only   # just the manifest (no 765MB dump)
python3 tools/extract_sbsave.py --get 7K3NYEL6            # print a program to stdout
python3 tools/extract_sbsave.py --get 12C3NWQE/3DSCLOSEGAME   # a project member
```

Pick candidates from `INDEX.json` (e.g. small `type:"TXT"` entries, high `refcount`)
without unpacking the whole tree. `encoding:"sb-bytes"` flags the few files that aren't
clean UTF-8 (SmileBASIC private-use glyphs) — the bytes are preserved verbatim.

## Caveats

- **Not goldens.** These are *inputs*, not verified expectations. Expected behavior must
  still come from real SB3 via the oracle (see the confidence ladder in `prd/README.md`).
  A program parsing/running cleanly is a smoke signal, not conformance.
- Community code: assorted SB versions, dialects, and quality. Many need graphics/sprites/
  audio (M2+) or files (M6) to run — they're not all M1-runnable.
- Scraped third-party content; treat as test fixtures, redistribute with care.
