"""Phase-A oracle: capture ground truth from real SmileBASIC 3.6.0 in Citra/Azahar.

Submodules:
  citra_rpc   - thin wrapper over tools/citra.py (process discovery + typed reads)
  extdata     - inject test programs, capture stdout/values/errors
  framebuffer - read the top/bottom screen framebuffers for pixel-diffing
  audio       - capture emulator audio output for sample/spectral diffing
"""
