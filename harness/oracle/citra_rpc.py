"""Typed wrapper over the Citra/Azahar scripting RPC (`tools/citra.py`).

Adds the repo's `tools/` to `sys.path`, re-exports `Citra`, and adds small helpers:
SmileBASIC process discovery and typed little-endian memory reads.

Works against Citra or Azahar (identical protocol). The emulator's scripting/RPC
server must be enabled, listening on UDP 45987.
"""
import struct
import sys
from pathlib import Path

_TOOLS = Path(__file__).resolve().parents[2] / "tools"
if str(_TOOLS) not in sys.path:
    sys.path.insert(0, str(_TOOLS))

from citra import Citra  # noqa: E402  (re-exported)

# SmileBASIC 3.6.0 (CIA update) title id; image base from the disassembly.
SMILEBASIC_TITLE_ID = 0x0004000E0016DE00
IMAGE_BASE = 0x00100000


def connect_smilebasic(address="127.0.0.1"):
    """Connect and select the running SmileBASIC process. Returns the `Citra` client
    (already `set_process`-ed) or raises if SmileBASIC isn't running."""
    c = Citra(address)
    for pid, (title_id, name) in c.process_list().items():
        if title_id == SMILEBASIC_TITLE_ID or "smile" in name.lower():
            c.set_process(pid)
            return c
    raise RuntimeError("SmileBASIC process not found — is it running in the emulator?")


def read_u32(c, addr):
    return struct.unpack("<I", c.read_memory(addr, 4))[0]


def read_s32(c, addr):
    return struct.unpack("<i", c.read_memory(addr, 4))[0]


def read_f64(c, addr):
    return struct.unpack("<d", c.read_memory(addr, 8))[0]


def read_utf16(c, addr, max_units=256):
    """Read a NUL-terminated UTF-16LE string (SmileBASIC's string encoding)."""
    raw = c.read_memory(addr, max_units * 2)
    units = struct.unpack(f"<{max_units}H", raw)
    out = []
    for u in units:
        if u == 0:
            break
        out.append(u)
    return "".join(chr(u) for u in out)


# TODO(spike): RE the addresses of ERRNUM/ERRLINE, the console grid, and the
# framebuffers from the disassembly so the oracle can read them directly.
ERRNUM_ADDR = None  # to be discovered
ERRLINE_ADDR = None
