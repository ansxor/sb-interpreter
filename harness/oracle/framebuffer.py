"""Oracle graphics capture — how a real-SB framebuffer becomes a committed golden (M2-T5).

The original RPC plan (read a live framebuffer address out of the emulator) is DEAD: Azahar
has no InputRedirection and the RPC crashes SB under load (see the sb-oracle skill README).
The working path, verified pixel-exact on real SB 3.6.0 (O-T6), is the **GRP-save** route:

    1. A program draws on a GRP page, then `SAVE "GRP<n>:NAME"` writes the whole 512×512
       RGBA5551 page out as an extdata file.
    2. `.claude/skills/sb-oracle/tools/run_case.py grp` pulls that file off disk and
       `sb_grp.py` decodes it (28-byte PCBN header + 512×512 RGBA5551 LE) to RGBA8888,
       cropping to the visible top-left 400×240, with **shift** 5→8 expansion (v<<3) so it
       matches `sb-render`'s `expand5` / the device color constants (e.g. #WHITE=&HFFF8F8F8).
    3. The cropped PNG is committed under `harness/corpus/golden/gfx/<name>.png` next to its
       drawing program `<name>.sb3`.

`harness/diff/replay.py` then pixel-diffs each committed golden against `sb-run --grp`
output — hermetically, no emulator (the golden is a frozen fixture). Composite/sprite/BG
display (the full screen, not a single GRP page) is captured via the skill's `screenshot`
(Ctrl+P) instead; that path is M3-T6.

This module is documentation + the canonical visible dimensions; the decode itself lives in
the oracle skill (`sb_grp.py`) so the harvest tooling stays in one place.
"""

# Visible draw area per screen (the GRP page is 512×512; only this top-left crop is shown).
TOP_W, TOP_H = 400, 240
BOTTOM_W, BOTTOM_H = 320, 240
