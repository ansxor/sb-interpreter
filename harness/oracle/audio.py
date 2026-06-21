"""Capture emulator audio output for diffing against `sb-audio` (Phase A, M5).

SmileBASIC's BGM/BEEP/TALK output is captured from the emulator (audio dump / loopback)
and compared to our synth by sample and/or spectral distance. This is where the project
gets the most leverage over `osb`, whose audio is stubbed.
"""


def capture(seconds: float):
    """Capture `seconds` of emulator audio as PCM samples. Stub (M5)."""
    raise NotImplementedError("oracle.audio.capture: implemented in milestone M5")
