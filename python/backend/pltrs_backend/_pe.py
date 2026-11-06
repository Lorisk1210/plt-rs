"""
The Windows platform tag a PE library (a ``.pyd``) earns, read from the
library itself: the architecture of its COFF header, and the DLLs it
imports, which must keep it to CPython's stable ABI: ``python3.dll``,
never a ``python3<minor>.dll`` that only one Python version has, and no
debug C runtime, which only machines with Visual Studio have.
"""

import re
import struct

#: COFF ``Machine`` values, by the architecture names of Rust targets.
MACHINES = {0x8664: "x86_64", 0xAA64: "aarch64", 0x14C: "i686"}

#: The platform tags of the architectures.
TAGS = {"x86_64": "win_amd64", "aarch64": "win_arm64", "i686": "win32"}

#: DLLs a stable-ABI extension for every Windows must not import.
FORBIDDEN = re.compile(r"python3\d+(_d)?\.dll|(vcruntime|msvcp)\d+d(_\d+)?\.dll|ucrtbased\.dll")


def _rva_to_offset(sections, rva):
    """The file offset of the relative virtual address *rva*."""
    for address, size, raw in sections:
        if address <= rva < address + size:
            return rva - address + raw
    raise ValueError(f"pltrs: the address {rva:#x} is in no section of the extension")


def _string(data, offset):
    """The NUL-terminated ASCII string at *offset*."""
    return data[offset:data.index(b"\0", offset)].decode("ascii")


def parse(data):
    """The architecture of the PE file *data* and the DLLs its import
    directory names."""
    if data[:2] != b"MZ":
        raise ValueError("pltrs: the extension is not a PE file")
    header, = struct.unpack_from("<I", data, 0x3C)
    if data[header:header + 4] != b"PE\0\0":
        raise ValueError("pltrs: the extension is not a PE file")
    machine, count, _, _, _, optional_size, _ = struct.unpack_from("<HHIIIHH", data, header + 4)
    optional = header + 24
    magic, = struct.unpack_from("<H", data, optional)
    # The data directories follow the PE32 (0x10b) or PE32+ (0x20b) fields.
    directories = optional + (96 if magic == 0x10B else 112)
    import_rva, _ = struct.unpack_from("<II", data, directories + 8)
    sections = []
    for index in range(count):
        entry = optional + optional_size + 40 * index
        virtual_size, address, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
        sections.append((address, max(virtual_size, raw_size), raw))
    dlls = []
    if import_rva:
        pos = _rva_to_offset(sections, import_rva)
        while True:
            lookup, _, _, name, thunks = struct.unpack_from("<IIIII", data, pos)
            if not (lookup or name or thunks):
                break
            dlls.append(_string(data, _rva_to_offset(sections, name)))
            pos += 20
    return MACHINES.get(machine, f"machine {machine:#x}"), dlls


def windows_tag(arch, library):
    """The platform tag of the Windows library whose bytes are *library*,
    built for the Rust architecture *arch*."""
    if arch not in TAGS:
        raise ValueError(f"pltrs: no wheel platform for Windows on {arch}")
    found, dlls = parse(library)
    if found != arch:
        raise ValueError(f"pltrs: the extension is built for {found}, not {arch}")
    forbidden = [dll for dll in dlls if FORBIDDEN.fullmatch(dll.lower())]
    if forbidden:
        raise RuntimeError(
            f"pltrs: the extension imports {', '.join(forbidden)}, which ties it to one "
            "Python or to Visual Studio's debug runtime")
    return TAGS[arch]
