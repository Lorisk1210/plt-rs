"""numpy's Windows wheels under Wine 10.0, whose UCRT lacks the complex
number accessors (`crealf`, `cimagf`, ...) numpy calls when imported:
their imports of the UCRT's math API set are pointed at a shim DLL
(`wine.sh python` builds it) that has those and forwards the rest.

    wine_numpy.py def SITE_PACKAGES OUT.def   write the shim's exports, print its file name
    wine_numpy.py patch SITE_PACKAGES         point numpy's modules at the shim

The shim goes next to python.exe, where Windows finds the DLLs of every
extension module.

Only the standard library is used: it runs on the host's Python.
"""

import os
import struct
import sys

API_SET = b"api-ms-win-crt-math-l1-1-0.dll"
# The same length, so that the name is replaced in place.
SHIM = b"crtmath-shim-for-wine-0000.dll"
# What the shim implements itself; the rest goes to ucrtbase.
OWN = {"crealf", "cimagf", "creal", "cimag", "creall", "cimagl"}


def imports(path):
    """The functions the PE32+ file `path` imports, by DLL name."""
    data = open(path, "rb").read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    sections_count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    # The import table is the second data directory of a PE32+ header.
    import_rva = struct.unpack_from("<I", data, optional + 120)[0]
    sections = []
    for i in range(sections_count):
        header = optional + optional_size + 40 * i
        virtual_size, address, raw_size, raw = struct.unpack_from("<IIII", data, header + 8)
        sections.append((address, max(virtual_size, raw_size), raw))

    def offset(rva):
        for address, size, raw in sections:
            if address <= rva < address + size:
                return rva - address + raw
        raise ValueError(f"RVA {rva:#x} is in no section")

    def name_at(rva):
        start = offset(rva)
        return data[start:data.index(b"\0", start)].decode()

    found = {}
    descriptor = offset(import_rva)
    while True:
        lookup, _, _, name_rva, thunks = struct.unpack_from("<IIIII", data, descriptor)
        if name_rva == 0:
            return found
        names = []
        entry = offset(lookup or thunks)
        while (value := struct.unpack_from("<Q", data, entry)[0]) != 0:
            if not value >> 63:  # imported by name, not ordinal
                names.append(name_at(value + 2))
            entry += 8
        found[name_at(name_rva).lower()] = names
        descriptor += 20


def modules(site):
    """numpy's extension modules under `site`."""
    for directory, _, files in os.walk(os.path.join(site, "numpy")):
        yield from (os.path.join(directory, f) for f in files if f.endswith(".pyd"))


def write_def(site, out):
    """Writes the shim's module definition: every math function numpy
    imports, forwarded to ucrtbase but for the shim's own."""
    names = set()
    for path in modules(site):
        for dll, imported in imports(path).items():
            if dll in (API_SET.decode(), SHIM.decode()):
                names.update(imported)
    with open(out, "w") as definition:
        definition.write(f"LIBRARY {SHIM.decode()}\nEXPORTS\n")
        for name in sorted(names):
            target = "" if name in OWN else f" = ucrtbase.{name}"
            definition.write(f"  {name}{target}\n")


def patch(site):
    """Points numpy's modules at the shim."""
    for path in modules(site):
        data = open(path, "rb").read()
        if API_SET in data:
            open(path, "wb").write(data.replace(API_SET, SHIM))


def main():
    match sys.argv[1:]:
        case ["def", site, out]:
            write_def(site, out)
            print(SHIM.decode())
        case ["patch", site]:
            patch(site)
        case _:
            sys.exit(__doc__)


main()
