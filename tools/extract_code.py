#!/usr/bin/env python3
"""Extract ARM .code from 3DS CXI/CIA using correct NCCH/ExeFS parsing."""
import struct
from pathlib import Path

MEDIA = 0x200


def read_u32(data, off):
    return struct.unpack_from('<I', data, off)[0]


def read_u64(data, off):
    return struct.unpack_from('<Q', data, off)[0]


def parse_exefs(exefs, base_path, label):
    """Parse ExeFS — file headers at offset 0, ARM .code typically at +0x200."""
    files = {}
    for i in range(10):
        off = i * 0x10
        if off + 16 > len(exefs):
            break
        name = exefs[off:off + 8].rstrip(b'\x00').decode('ascii', errors='replace')
        if not name:
            continue
        # In ExeFS, offset is in bytes? Let me figure this out empirically.
        hdr_offset = read_u32(exefs, off + 8)
        hdr_size = read_u32(exefs, off + 12)
        if hdr_size == 0:
            continue
        files[name] = {'hdr_offset': hdr_offset, 'size': hdr_size}

    print(f"  ExeFS files: {list(files.keys())}")
    for name, info in files.items():
        print(f"    {name}: hdr_offset=0x{info['hdr_offset']:X}, size={info['size']:,}")

    # The ExeFS header is 0x200 bytes (file headers + hashes).
    # .code ALWAYS starts at data_offset=0 in the header and lives at ExeFS+0x200.
    # The header offset field appears to be in bytes relative to ExeFS data start (+0x200).
    if '.code' in files:
        code_info = files['.code']
        # code starts at ExeFS + 0x200 (after headers)
        code_start = 0x200
        code_data = exefs[code_start:code_start + code_info['size']]
        out_path = base_path / f"{label}_code.bin"
        out_path.write_bytes(code_data)
        print(f"  ✓ Extracted .code: {out_path} ({len(code_data):,} bytes)")

        # Verify: first instruction should look like ARM
        if len(code_data) >= 4:
            insn = read_u32(code_data, 0)
            print(f"    First word: 0x{insn:08X}")
        return str(out_path)

    print(f"  ⚠ No .code found in ExeFS")
    return None


def process_ncch(data, base_path, label):
    """Parse NCCH and extract .code from ExeFS."""
    if len(data) < 0x200:
        print(f"  Too small for NCCH")
        return None

    magic = data[0x100:0x104]
    if magic != b'NCCH':
        print(f"  Not NCCH (magic={magic!r})")
        return None

    exefs_off = read_u32(data, 0x1A0) * MEDIA
    exefs_sz = read_u32(data, 0x1A4) * MEDIA
    exthdr_sz = read_u32(data, 0x180)
    print(f"  NCCH: ExeFS at 0x{exefs_off:X} size 0x{exefs_sz:X}, ExHeader={exthdr_sz}")

    if exefs_off == 0 or exefs_sz == 0:
        print(f"  No ExeFS")
        return None

    exefs = data[exefs_off:exefs_off + exefs_sz]
    return parse_exefs(exefs, base_path, label)


def process_cia(data, base_path):
    """Parse CIA container."""
    hdr_sz = read_u32(data, 0x00)
    cert_sz = read_u32(data, 0x08)
    ticket_sz = read_u32(data, 0x0C)
    tmd_sz = read_u32(data, 0x10)
    content_sz = read_u64(data, 0x18)

    content_off = hdr_sz + cert_sz + ticket_sz + tmd_sz
    content_off = (content_off + 63) & ~63

    print(f"  CIA: header=0x{hdr_sz:X} cert={cert_sz} ticket={ticket_sz} tmd={tmd_sz}")
    print(f"  Content at 0x{content_off:X}, size={content_sz:,}")

    # Scan for NCCH magics in content area
    content = data[content_off:content_off + content_sz]
    results = []
    idx = 0
    ncch_count = 0
    while True:
        pos = content.find(b'NCCH', idx)
        if pos == -1:
            break
        ncch_data_offset = pos - 0x100  # NCCH starts 0x100 bytes before magic
        if ncch_data_offset >= 0:
            ncch = content[ncch_data_offset:]
            label = f"cia_ncch{ncch_count}"
            print(f"\n  --- NCCH #{ncch_count} at content+0x{ncch_data_offset:X} ---")
            result = process_ncch(ncch, base_path, label)
            if result:
                results.append(result)
            ncch_count += 1
        idx = pos + 1

    return results


def main():
    cwd = Path("/Users/darien/Workspace/sb-interpreter")

    for f in sorted(cwd.glob("*.cxi")) + sorted(cwd.glob("*.cia")):
        print(f"\n{'='*60}")
        print(f"{f.name} ({f.stat().st_size:,} bytes)")
        data = f.read_bytes()

        if f.suffix.lower() == '.cia':
            process_cia(data, cwd)
        else:
            process_ncch(data, cwd, 'cxi_base')


if __name__ == '__main__':
    main()
