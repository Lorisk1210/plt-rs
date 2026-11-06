"""
The macOS platform tag a Mach-O library earns, read from the library
itself as delocate reads it: the architectures of its slices (one, or
two in a universal binary), the oldest macOS each slice runs on (its
``LC_BUILD_VERSION`` or ``LC_VERSION_MIN_MACOSX``, which rustc sets from
``MACOSX_DEPLOYMENT_TARGET`` or its default: 10.12 on x86_64, 11.0 on
arm64) and the libraries it loads, which must be the system's.

Tags name ``macosx_<major>_<minor>_<arch>``: ``x86_64``, ``arm64``, or
``universal2`` for both. From macOS 11 on, pip only knows ``<major>_0``,
so a library needing 11.3 is tagged ``12_0``, never ``11_0``. A
universal2 tag stands for both slices: its version is the x86_64
slice's, raised to the arm64 slice's when that needs more than 11.0, the
first macOS on arm64.
"""

import dataclasses
import struct

#: ``cputype`` values.
CPU_TYPES = {0x01000007: "x86_64", 0x0100000C: "arm64"}

#: Load commands.
LC_VERSION_MIN_MACOSX, LC_BUILD_VERSION = 0x24, 0x32
LOAD_DYLIB_COMMANDS = {0xC, 0x20, 0x80000018, 0x8000001F, 0x80000023}

#: ``PLATFORM_MACOS`` of ``LC_BUILD_VERSION``.
PLATFORM_MACOS = 1

#: Where the libraries every Mac has live.
SYSTEM_PREFIXES = ("/usr/lib/", "/System/Library/")

#: The first macOS on arm64.
ARM64_FIRST = (11, 0)


@dataclasses.dataclass(frozen=True)
class Slice:
    """One architecture of a Mach-O file: the oldest macOS it runs on
    (None if it does not say) and the libraries it loads."""

    arch: str
    minimum: tuple | None
    dylibs: tuple


def _thin(data, offset):
    """The :class:`Slice` of the 64-bit Mach-O image at *offset*."""
    magic, cputype, _, _, ncmds, _ = struct.unpack_from("<IiiIII", data, offset)
    if magic != 0xFEEDFACF:
        raise ValueError("pltrs: the extension is not a 64-bit Mach-O library")
    minimum, dylibs = None, []
    pos = offset + 32
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", data, pos)
        if cmd == LC_BUILD_VERSION:
            platform, minos = struct.unpack_from("<II", data, pos + 8)
            if platform == PLATFORM_MACOS:
                minimum = (minos >> 16, (minos >> 8) & 0xFF)
        elif cmd == LC_VERSION_MIN_MACOSX and minimum is None:
            version, = struct.unpack_from("<I", data, pos + 8)
            minimum = (version >> 16, (version >> 8) & 0xFF)
        elif cmd in LOAD_DYLIB_COMMANDS:
            name_offset, = struct.unpack_from("<I", data, pos + 8)
            start = pos + name_offset
            dylibs.append(data[start:data.index(b"\0", start, pos + size)].decode())
        pos += size
    return Slice(CPU_TYPES.get(cputype, f"cputype {cputype:#x}"), minimum, tuple(dylibs))


def slices(data):
    """The :class:`Slice` of each architecture of the Mach-O file *data*:
    one for a thin file, one per architecture of a universal one."""
    magic, = struct.unpack_from(">I", data, 0)
    if magic in (0xCAFEBABE, 0xCAFEBABF):
        count, = struct.unpack_from(">I", data, 4)
        wide = magic == 0xCAFEBABF
        layout, step = (">iiQQII", 32) if wide else (">iiIII", 20)
        return [_thin(data, struct.unpack_from(layout, data, 8 + index * step)[2])
                for index in range(count)]
    return [_thin(data, 0)]


def _tag_version(version):
    """The ``<major>_<minor>`` of a tag for a library needing *version*."""
    major, minor = version
    if major >= 11 and minor:
        major, minor = major + 1, 0
    return f"{major}_{minor}"


def macos_tag(arch, library):
    """
    The platform tag of the macOS library whose bytes are *library*,
    built for the Rust architecture *arch* (``x86_64``, ``aarch64``, or
    ``universal2`` for both); see the module's description.
    """
    found = {piece.arch: piece for piece in slices(library)}
    wanted = {"universal2": {"x86_64", "arm64"}, "aarch64": {"arm64"}}.get(arch, {arch})
    if set(found) != wanted:
        raise ValueError(f"pltrs: the extension holds {', '.join(sorted(found))}, "
                         f"not {', '.join(sorted(wanted))}")
    foreign = sorted({dylib for piece in found.values() for dylib in piece.dylibs
                      if not dylib.startswith(SYSTEM_PREFIXES)})
    if foreign:
        raise RuntimeError(f"pltrs: the extension loads libraries macOS lacks: {', '.join(foreign)}")
    lacking = sorted(name for name, piece in found.items() if piece.minimum is None)
    if lacking:
        raise ValueError(f"pltrs: the {', '.join(lacking)} extension names no macOS version")
    if "x86_64" in found:
        version = found["x86_64"].minimum
        if "arm64" in found and found["arm64"].minimum > ARM64_FIRST:
            version = max(version, found["arm64"].minimum)
    else:
        version = max(found["arm64"].minimum, ARM64_FIRST)
    name = "universal2" if len(found) == 2 else next(iter(found))
    return f"macosx_{_tag_version(version)}_{name}"
