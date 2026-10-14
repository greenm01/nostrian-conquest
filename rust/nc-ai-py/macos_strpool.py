"""Move a 64-bit Mach-O's string pool up to 8-byte alignment and re-sign it.

The pool is moved into the padding that follows it, before the code
signature or the end of __LINKEDIT; symbol name offsets are relative to the
pool, so nothing else changes. Aligned files are left alone.
"""

import struct
import subprocess
import sys

MH_MAGIC_64 = 0xFEEDFACF
LC_SEGMENT_64 = 0x19
LC_SYMTAB = 0x2
LC_CODE_SIGNATURE = 0x1D


def main(path):
    with open(path, "rb") as file:
        data = bytearray(file.read())
    if len(data) < 32 or struct.unpack_from("<I", data)[0] != MH_MAGIC_64:
        return
    ncmds = struct.unpack_from("<I", data, 16)[0]
    offset = 32
    symtab = signature = linkedit_end = None
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", data, offset)
        if cmd == LC_SYMTAB:
            symtab = offset
        elif cmd == LC_CODE_SIGNATURE:
            signature = struct.unpack_from("<I", data, offset + 8)[0]
        elif cmd == LC_SEGMENT_64 and data[offset + 8 : offset + 18] == b"__LINKEDIT":
            fileoff, filesize = struct.unpack_from("<QQ", data, offset + 40)
            linkedit_end = fileoff + filesize
        offset += size
    if symtab is None:
        return
    stroff, strsize = struct.unpack_from("<II", data, symtab + 16)
    pad = -stroff % 8
    if pad == 0:
        return
    limit = signature if signature is not None else linkedit_end
    if limit is None or stroff + strsize + pad > limit:
        sys.exit(f"{path}: no room to align the string pool at {stroff}")
    data[stroff + pad : stroff + pad + strsize] = data[stroff : stroff + strsize]
    data[stroff : stroff + pad] = bytes(pad)
    struct.pack_into("<I", data, symtab + 16, stroff + pad)
    with open(path, "wb") as file:
        file.write(data)
    if signature is not None:
        subprocess.run(["codesign", "-f", "-s", "-", path], check=True,
                       stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    main(sys.argv[1])
