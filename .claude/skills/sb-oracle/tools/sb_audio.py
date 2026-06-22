#!/usr/bin/env python3
"""Audio REFERENCE capture for SmileBASIC (O-T7) — best-effort, NON-deterministic.

Unlike graphics (a GRP page is a deterministic buffer we read off disk), SmileBASIC has NO way
to render audio to a file, and the only way out is the emulator's real-time video dump. Real-
time emulator audio is timing/mixing/sample-rate dependent, so this CANNOT be a frozen,
sample-exact CI golden. Use it for **manual ear-checks / loose spectral comparison only**.

The deterministic contract for audio (M5) lives elsewhere: MML -> note-event specs
(pitch/duration/volume/envelope) + synth parameter tables, authored from docs + disassembly,
with NO emulator. See prd/oracle.md (O-T7) and prd/M5.md.

Mechanism: Azahar `Tools > Dump Video` (toggle) records screen+audio via FFmpeg; we then
extract the audio track to WAV with ffmpeg. The menu toggle + its save dialog are driven by
osascript/cliclick. **This orchestration is live-UNTESTED** (kept off the running oracle to
avoid wedging it on the save dialog); verify the save-dialog step once before trusting it.
The `extract_wav` step is plain ffmpeg and is reliable.
"""
import glob
import os
import subprocess
import sys
import time

DUMPDIR = os.path.expanduser("~/Library/Application Support/Azahar/video_dumps")
PROC = "azahar"


def extract_wav(video_path, out_wav):
    """Extract the audio track of a dumped video to 16-bit PCM mono WAV (ffmpeg). Reliable."""
    subprocess.run(["ffmpeg", "-y", "-i", video_path, "-vn", "-acodec", "pcm_s16le",
                    "-ac", "1", out_wav], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return out_wav


def _menu_dump_video():
    """Click Azahar's Tools > Dump Video menu item (toggles dumping on/off)."""
    subprocess.run(["osascript", "-e",
                    f'tell application "System Events" to tell (first process whose name '
                    f'contains "{PROC}") to click menu item "Dump Video" of menu "Tools" '
                    f'of menu bar 1'])


def _newest_dump(before):
    files = set(glob.glob(os.path.join(DUMPDIR, "*"))) - before
    return max(files, key=os.path.getmtime) if files else None


def capture_audio(program, out_wav, seconds=6.0, run_trigger=None):
    """BEST-EFFORT, LIVE-UNTESTED reference capture. Writes `program` to slot P, toggles video
    dump on, runs the program (via `run_trigger()` — pass run_case._trigger_run), lets it play
    `seconds`, toggles dump off, then extracts the newest dump's audio to `out_wav`.

    The dump's save dialog defaults to videoDumpingPath; this presses Return to accept it. If
    your Azahar prompts differently, handle the dialog before relying on this. NON-deterministic
    — the WAV is a reference, never a committed CI golden."""
    import sb_extdata as X  # local import: only needed when actually capturing
    import sb_window as W
    X.write_file("P", program if program.endswith("\n") else program + "\n", "TXT")
    W.raise_window()
    time.sleep(0.5)
    before = set(glob.glob(os.path.join(DUMPDIR, "*")))
    _menu_dump_video()                 # start dump -> save dialog
    time.sleep(1.0)
    W.enter()                          # accept the default save path
    time.sleep(1.0)
    if run_trigger:
        run_trigger("P")               # LOAD"PRG0:P",0:RUN (plays the audio)
    time.sleep(seconds)
    _menu_dump_video()                 # stop dump (finalizes the file)
    time.sleep(2.0)
    vid = _newest_dump(before)
    if not vid:
        raise RuntimeError(f"no new dump file in {DUMPDIR} (did the menu/save dialog work?)")
    return extract_wav(vid, out_wav)


if __name__ == "__main__":
    a = sys.argv[1:]
    if len(a) >= 2 and a[0] == "extract":          # testable: extract WAV from an existing video
        print("wrote:", extract_wav(a[1], a[2] if len(a) > 2 else a[1] + ".wav"))
    else:
        print("usage: sb_audio.py extract <video> [out.wav]   "
              "(capture_audio() is the live orchestration — see module docstring)")
