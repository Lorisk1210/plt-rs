"""
Reading what an ELF shared library needs from the system: its
architecture, the libraries it names (``DT_NEEDED``), the symbol versions
it asks of them (``.gnu.version_r``) and the symbols it leaves undefined,
each with the version it was bound to (``.gnu.version``) and whether its
reference is weak.

Only the section headers, the dynamic section and the dynamic symbol
table are read, which stripping keeps.
"""

import dataclasses
import struct

#: The architectures of ``e_machine`` values, named as manylinux tags
#: name them (``EM_PPC64`` is ``ppc64le`` in little-endian files).
MACHINES = {
    3: "i686", 40: "armv7l", 62: "x86_64", 183: "aarch64", 21: "ppc64",
    22: "s390x", 243: "riscv64", 258: "loongarch64",
}

#: Section types.
SHT_DYNAMIC, SHT_DYNSYM = 6, 11
SHT_GNU_VERNEED, SHT_GNU_VERSYM = 0x6FFFFFFE, 0x6FFFFFFF

#: ``VER_FLG_WEAK``: a version only weak references need.
VER_FLG_WEAK = 2

#: ``STB_WEAK``: a symbol's weak binding.
STB_WEAK = 2


@dataclasses.dataclass(frozen=True)
class Version:
    """A symbol version a library needs from another: ``GLIBC_2.17`` of
    ``libc.so.6``; *weak* when only weak references need it."""

    library: str
    name: str
    weak: bool


@dataclasses.dataclass(frozen=True)
class Undefined:
    """A symbol a library leaves for others to define, with the version
    it was bound to (None when unversioned)."""

    name: str
    version: Version | None
    weak: bool


@dataclasses.dataclass(frozen=True)
class Elf:
    """What a shared library needs: see the module's description."""

    arch: str
    needed: tuple
    versions: tuple
    undefined: tuple


class _Reader:
    """Fixed-size fields of an ELF file of one class and byte order."""

    def __init__(self, data):
        if data[:4] != b"\x7fELF":
            raise ValueError("pltrs: the extension is not an ELF file")
        self.data = data
        self.is64 = data[4] == 2
        self.end = "<" if data[5] == 1 else ">"

    def unpack(self, layout, offset):
        """The fields *layout* (``struct`` codes) describes at *offset*."""
        return struct.unpack_from(self.end + layout, self.data, offset)

    def string(self, section, offset):
        """The NUL-terminated string at *offset* of string table *section*."""
        start = section.offset + offset
        return self.data[start:self.data.index(b"\0", start)].decode()


@dataclasses.dataclass(frozen=True)
class _Section:
    kind: int
    offset: int
    size: int
    link: int
    info: int
    entsize: int


def _sections(reader):
    """The section headers."""
    if reader.is64:
        shoff, = reader.unpack("Q", 0x28)
        shentsize, shnum = reader.unpack("HH", 0x3A)
        layout = "IIQQQQIIQQ"
    else:
        shoff, = reader.unpack("I", 0x20)
        shentsize, shnum = reader.unpack("HH", 0x2E)
        layout = "IIIIIIIIII"
    sections = []
    for index in range(shnum):
        _, kind, _, _, offset, size, link, info, _, entsize = reader.unpack(
            layout, shoff + index * shentsize)
        sections.append(_Section(kind, offset, size, link, info, entsize))
    return sections


def _needed(reader, sections, dynamic):
    """The ``DT_NEEDED`` libraries of the dynamic section *dynamic*."""
    strings = sections[dynamic.link]
    step, layout = (16, "qQ") if reader.is64 else (8, "iI")
    needed = []
    for pos in range(dynamic.offset, dynamic.offset + dynamic.size, step):
        tag, value = reader.unpack(layout, pos)
        if tag == 0:  # DT_NULL
            break
        if tag == 1:  # DT_NEEDED
            needed.append(reader.string(strings, value))
    return needed


def _version_needs(reader, sections, verneed):
    """The versions of the ``SHT_GNU_verneed`` section *verneed*, by the
    index ``.gnu.version`` refers to them with."""
    strings = sections[verneed.link]
    versions = {}
    pos = verneed.offset
    for _ in range(verneed.info):
        _, count, file, aux, following = reader.unpack("HHIII", pos)
        library = reader.string(strings, file)
        apos = pos + aux
        for _ in range(count):
            _, flags, index, name, anext = reader.unpack("IHHII", apos)
            versions[index] = Version(library, reader.string(strings, name),
                                      bool(flags & VER_FLG_WEAK))
            apos += anext
        pos += following
    return versions


def _undefined(reader, sections, dynsym, versym, versions):
    """The undefined symbols of the dynamic symbol table *dynsym*, with
    their versions."""
    strings = sections[dynsym.link]
    layout, size = ("IBBHQQ", 24) if reader.is64 else ("IIIBBH", 16)
    found = []
    for index in range(1, dynsym.size // size):
        fields = reader.unpack(layout, dynsym.offset + index * size)
        if reader.is64:
            name, info, _, shndx, _, _ = fields
        else:
            name, _, _, info, _, shndx = fields
        if shndx != 0 or name == 0:  # defined here, or nameless
            continue
        version = None
        if versym is not None:
            index_version, = reader.unpack("H", versym.offset + 2 * index)
            version = versions.get(index_version & 0x7FFF)
        found.append(Undefined(reader.string(strings, name), version, info >> 4 == STB_WEAK))
    return found


def parse(data):
    """The :class:`Elf` description of the shared library *data* (bytes)."""
    reader = _Reader(data)
    machine, = reader.unpack("H", 0x12)
    arch = MACHINES.get(machine, f"machine {machine}")
    if arch == "ppc64" and reader.end == "<":
        arch = "ppc64le"
    sections = _sections(reader)
    by_kind = {}
    for section in sections:
        by_kind.setdefault(section.kind, section)
    dynamic = by_kind.get(SHT_DYNAMIC)
    dynsym = by_kind.get(SHT_DYNSYM)
    verneed = by_kind.get(SHT_GNU_VERNEED)
    versions = _version_needs(reader, sections, verneed) if verneed else {}
    return Elf(
        arch=arch,
        needed=tuple(_needed(reader, sections, dynamic) if dynamic else ()),
        versions=tuple(versions.values()),
        undefined=tuple(_undefined(reader, sections, dynsym, by_kind.get(SHT_GNU_VERSYM),
                                   versions) if dynsym else ()),
    )
