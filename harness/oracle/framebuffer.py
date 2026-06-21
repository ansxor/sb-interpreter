"""Read the SmileBASIC screen framebuffers from the emulator for pixel-diffing.

The composited buffer our renderer produces (`sb_render::Framebuffer`) is diffed
against what real SB3 displays, read here via the RPC. Output is RGBA8888 to match
`sb-render`.

Spike: RE the framebuffer base address(es) + pixel format from the disassembly. The
3DS upper screen is 400x240, lower 320x240; the on-device format is typically a tiled
RGB(A) surface, so a detiling step may be required before comparison.
"""
TOP_W, TOP_H = 400, 240
BOTTOM_W, BOTTOM_H = 320, 240

TOP_FB_ADDR = None  # TODO(spike): discover from the disassembly
BOTTOM_FB_ADDR = None


def read_framebuffer(c, addr, width, height):
    """Read `width`x`height` pixels at `addr` and return RGBA8888 bytes.

    Stub: the decode/detile depends on the as-yet-unknown on-device pixel format.
    """
    raise NotImplementedError(
        "oracle.framebuffer.read_framebuffer: pending framebuffer-address + "
        "pixel-format spike"
    )
