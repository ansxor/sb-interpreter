"""Capture text / values / errors from real SmileBASIC (Phase A).

Strategy (see plan): write each test program plus a SmileBASIC harness-loader program
into the emulator's SB extdata, run them, and read results back. stdout is scraped
inside SB via CHKCHR(x,y); errors are read from ERRNUM/ERRLINE via the RPC after a halt.

Open spikes (resolve before this is usable):
  - autorun: how to auto-start a program in SB3 under emulation
  - extdata container format: SB3's on-disk format for stored programs/files
  - stdout capture: CHKCHR grid scrape vs RPC console-memory read
  - error capture: read ERRNUM/ERRLINE via RPC after the program halts
"""
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class OracleResult:
    """Ground truth captured from one program run."""
    stdout: str = ""
    errnum: Optional[int] = None
    errline: Optional[int] = None
    values: dict = field(default_factory=dict)


def run_program(code: str, inputs=None) -> OracleResult:
    """Run `code` on the real SB3 oracle and return captured output.

    Not implemented yet — depends on the autorun + extdata-format spikes. Raising
    keeps Phase A honest: nothing silently fabricates ground truth.
    """
    raise NotImplementedError(
        "oracle.extdata.run_program: pending autorun + extdata-format spikes "
        "(see harness/oracle/extdata.py docstring and the project plan)"
    )
