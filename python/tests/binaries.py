"""
Minimal ELF, Mach-O and PE shared libraries, made byte by byte for the
build backend's tests (``test_build_backend_tags.py``): only the parts
its checks read (the dynamic section, symbols and their versions; load
commands; the import directory), so that every rule is tested on any
system, with libraries no toolchain would make.
"""

import struct


def _strings(names):
    """A string table holding *names*, and each name's offset in it."""
    table, offsets = bytearray(b"\0"), {}
    for name in names:
        if name not in offsets:
            offsets[name] = len(table)
            table += name.encode() + b"\0"
    return bytes(table), offsets


def elf(machine=62, needed=("libc.so.6",), versions=(), symbols=(), little=True):
    """
    A 64-bit ELF shared library for ``e_machine`` *machine* naming the
    libraries *needed* and the symbol *versions* (``(library, name,
    weak)``); *symbols* are its undefined symbols, ``(name, version name
    or None, weak)``, a version naming one of *versions*.
    """
    end = "<" if little else ">"
    libraries = list(dict.fromkeys(library for library, _, _ in versions))
    strings, offset = _strings([*needed, *libraries, *(name for _, name, _ in versions),
                                *(name for name, _, _ in symbols)])
    index = {name: 2 + i for i, (_, name, _) in enumerate(versions)}
    dynsym = bytes(24) + b"".join(
        struct.pack(end + "IBBHQQ", offset[name], (2 if weak else 1) << 4 | 2, 0, 0, 0, 0)
        for name, _, weak in symbols)
    versym = struct.pack(end + "H", 0) + b"".join(
        struct.pack(end + "H", index[version] if version else 1) for _, version, _ in symbols)
    verneed = bytearray()
    for number, library in enumerate(libraries):
        own = [(name, weak) for lib, name, weak in versions if lib == library]
        following = 16 + 16 * len(own) if number < len(libraries) - 1 else 0
        verneed += struct.pack(end + "HHIII", 1, len(own), offset[library], 16, following)
        for position, (name, weak) in enumerate(own):
            verneed += struct.pack(end + "IHHII", 0, 2 if weak else 0, index[name], offset[name],
                                   16 if position < len(own) - 1 else 0)
    dynamic = b"".join(struct.pack(end + "qQ", 1, offset[name]) for name in needed)
    dynamic += struct.pack(end + "qQ", 0, 0)
    # Sections: null, .dynstr, .dynsym, .gnu.version, .gnu.version_r, .dynamic.
    bodies = [strings, dynsym, versym, bytes(verneed), dynamic]
    kinds = [(3, 0, 0), (11, 1, 0), (0x6FFFFFFF, 2, 0), (0x6FFFFFFE, 1, len(libraries)),
             (6, 1, 0)]
    data = bytearray(64)
    places = []
    for body in bodies:
        places.append(len(data))
        data += body
    shoff = len(data)
    data += bytes(64)
    for (kind, link, info), body, place in zip(kinds, bodies, places):
        data += struct.pack(end + "IIQQQQIIQQ", 0, kind, 0, 0, place, len(body), link, info, 8, 0)
    ident = b"\x7fELF" + bytes([2, 1 if little else 2, 1]) + bytes(9)
    data[:64] = struct.pack(end + "16sHHIQQQIHHHHHH", ident, 3, machine, 1, 0, 0, shoff, 0,
                            64, 56, 0, 64, 1 + len(bodies), 0)
    return bytes(data)


#: Mach-O CPU types.
X86_64, ARM64 = 0x01000007, 0x0100000C


def macho(cputype, minimum=(11, 0), dylibs=("/usr/lib/libSystem.B.dylib",), legacy=False):
    """A 64-bit Mach-O library for *cputype* running on macOS *minimum*
    or later (said by ``LC_VERSION_MIN_MACOSX`` when *legacy*, else by
    ``LC_BUILD_VERSION``; None says nothing) and loading *dylibs*."""
    commands = []
    if minimum is not None:
        version = minimum[0] << 16 | minimum[1] << 8
        commands.append(struct.pack("<IIII", 0x24, 16, version, version) if legacy
                        else struct.pack("<IIIIII", 0x32, 24, 1, version, version, 0))
    for dylib in dylibs:
        name = dylib.encode() + b"\0"
        size = (24 + len(name) + 7) // 8 * 8
        commands.append(struct.pack("<IIIIII", 0xC, size, 24, 0, 0, 0)
                        + name.ljust(size - 24, b"\0"))
    body = b"".join(commands)
    return struct.pack("<IiiIIIII", 0xFEEDFACF, cputype, 0, 6, len(commands), len(body), 0, 0) + body


def fat(*slices):
    """A universal Mach-O file of the thin *slices* (``(cputype,
    bytes)``), each on a page of its own."""
    header = struct.pack(">II", 0xCAFEBABE, len(slices))
    data = bytearray(4096)
    entries = b""
    for cputype, thin in slices:
        offset = len(data)
        entries += struct.pack(">iiIII", cputype, 0, offset, len(thin), 12)
        data += thin.ljust((len(thin) + 4095) // 4096 * 4096, b"\0")
    data[:len(header) + len(entries)] = header + entries
    return bytes(data)


def pe(machine=0x8664, dlls=("python3.dll", "KERNEL32.dll")):
    """A PE32+ library for COFF *machine* importing *dlls*."""
    names = b""
    name_offsets = []
    table_size = 20 * (len(dlls) + 1)
    for dll in dlls:
        name_offsets.append(table_size + len(names))
        names += dll.encode() + b"\0"
    section_rva, section_raw = 0x1000, 0x200
    imports = b"".join(struct.pack("<IIIII", 1, 0, 0, section_rva + offset, 1)
                       for offset in name_offsets) + bytes(20) + names
    optional = bytearray(240)
    optional[:2] = struct.pack("<H", 0x20B)
    optional[120:128] = struct.pack("<II", section_rva, len(imports))
    section = struct.pack("<8sIIIIIIHHI", b".idata", len(imports), section_rva, len(imports),
                          section_raw, 0, 0, 0, 0, 0)
    header = b"PE\0\0" + struct.pack("<HHIIIHH", machine, 1, 0, 0, 0, len(optional), 0x2022)
    dos = b"MZ".ljust(0x3C, b"\0") + struct.pack("<I", 0x40)
    data = (dos + header + bytes(optional) + section).ljust(section_raw, b"\0")
    return data + imports
