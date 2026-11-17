"""
Writes ``crates/pltrs-python/symbol-order.txt``, the order in which the
linker lays out the extension's functions (see ``build.rs``): first the
functions every program runs, then those fewer programs run.

    .venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py [--jobs N]

A process maps the code of the extension page by page as it runs it, and
the kernel maps the pages around each one it faults in with it. Laid out in
the compiler's order, the code a small figure runs is spread over most of
the library, so that a process saving a small PNG held 11.4 MiB of it;
grouped, it holds 5.0 MiB: the code every program runs is mapped once,
and the rest only where a program runs it.

The tool runs the benchmark scripts (``benchmarks/python``, their imports
of matplotlib read as pltrs's), each saving a PNG (the animation its
JavaScript player, ``line`` and ``mid_size`` also SVG, PDF, PostScript and
EPS), and a run that only imports ``pltrs.pyplot``, with the extension
built in ``python/pltrs`` (``cargo xtask python --release``), and records which of its code pages each run reads. For that each run loads a copy of the
extension that is in no page cache, with read-ahead turned off
(``MADV_RANDOM``), so that the pages the process maps are those it ran,
and reads them from ``/proc/self/pagemap``. The functions on those pages
are written in the order of how many runs read them, then in the order in
which they lie now. Linux only; the runs write to ``target/symbol-order``.

The file names the functions as rustc mangles them in the extension
profile's build (``cargo xtask python --release``, the wheels; cargo names
a crate's functions after the profile), which changes as the code does; a
function it does not name is laid out after those it does.
Run the tool again (twice, the second time with the extension linked in
the first run's order) when the code has changed much.
"""

import argparse
import bisect
import concurrent.futures
import ctypes
import importlib.machinery
import importlib.util
import mmap
import os
import pathlib
import runpy
import shutil
import struct
import subprocess
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[3]
ORDER = ROOT / "crates/pltrs-python/symbol-order.txt"
WORK = ROOT / "target/symbol-order"
#: The formats scripts save in, where not PNG alone: the animation's
#: JavaScript player (``to_jshtml``), and every document format.
FORMATS = {"animation_30": ("html",), "line": ("png", "svg", "pdf", "ps", "eps"),
           "mid_size": ("png", "svg", "pdf", "ps", "eps")}
PAGE = 4096


def _mappings(path):
    """The mappings of the file *path* in this process: ``(start, end,
    file offset, permissions)``."""
    with open("/proc/self/maps") as maps:
        for line in maps:
            fields = line.split()
            if len(fields) >= 6 and fields[5] == path:
                start, end = (int(value, 16) for value in fields[0].split("-"))
                yield start, end, int(fields[2], 16), fields[1]


def _uncached_copy(extension, directory):
    """A copy of *extension* in *directory* that no page cache holds."""
    copy = directory / f"_native-{os.getpid()}{extension.suffix}"
    shutil.copyfile(extension, copy)
    descriptor = os.open(copy, os.O_RDONLY)
    try:
        os.fsync(descriptor)
        os.posix_fadvise(descriptor, 0, 0, os.POSIX_FADV_DONTNEED)
    finally:
        os.close(descriptor)
    return copy


class _Redirect:
    """Imports the package's ``_native`` from *path*."""

    def __init__(self, path):
        self.path = str(path)

    def find_spec(self, name, path=None, target=None):
        if name.rpartition(".")[2] != "_native":
            return None
        loader = importlib.machinery.ExtensionFileLoader(name, self.path)
        return importlib.util.spec_from_file_location(name, self.path, loader=loader)


def trace(out, script, arguments):
    """Run *script* with *arguments* and write the file offsets of the
    extension's code pages it read to *out*, one per line."""
    package = pathlib.Path(importlib.util.find_spec("pltrs").origin).parent
    extension = next(package.glob("_native*.so"))
    copy = _uncached_copy(extension, pathlib.Path(out).parent)
    try:
        ctypes.CDLL(str(copy), mode=os.RTLD_NOW)
        libc = ctypes.CDLL(None, use_errno=True)
        libc.madvise.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_int]
        for start, end, _, _ in _mappings(str(copy)):
            libc.madvise(start, end - start, mmap.MADV_RANDOM)
        sys.meta_path.insert(0, _Redirect(copy))
        sys.path.insert(0, str(pathlib.Path(script).parent))
        sys.argv = [script, *arguments]
        runpy.run_path(script, run_name="__main__")
        with open("/proc/self/pagemap", "rb") as pagemap, open(out, "w") as written:
            for start, end, offset, permissions in _mappings(str(copy)):
                if "x" not in permissions:
                    continue
                pagemap.seek(start // PAGE * 8)
                count = (end - start) // PAGE
                entries = struct.unpack(f"{count}Q", pagemap.read(count * 8))
                written.writelines(f"{offset + index * PAGE}\n"
                                   for index, entry in enumerate(entries) if entry >> 63)
    finally:
        copy.unlink()


#: The section type of an ELF symbol table (``SHT_SYMTAB``).
SYMBOL_TABLE = 2


def _sections(data):
    """The section headers of the ELF file whose bytes are *data*."""
    (section_offset,) = struct.unpack_from("<Q", data, 0x28)
    entry_size, count = struct.unpack_from("<HH", data, 0x3A)
    return [struct.unpack_from("<IIQQQQIIQQ", data, section_offset + i * entry_size)
            for i in range(count)]


def has_symbol_table(library):
    """Whether the ELF shared library *library* keeps its symbol table
    (``.symtab``), which stripping removes."""
    sections = _sections(pathlib.Path(library).read_bytes())
    return any(section[1] == SYMBOL_TABLE for section in sections)


def functions(library):
    """The functions of the ELF shared library *library*: ``(file offset,
    size, name)``, by offset."""
    data = pathlib.Path(library).read_bytes()
    sections = _sections(data)
    count = len(sections)
    symbol_table = next(s for s in sections if s[1] == SYMBOL_TABLE)
    strings = sections[symbol_table[6]]
    found = []
    for at in range(symbol_table[4], symbol_table[4] + symbol_table[5], 24):
        name, info, _, index, value, size = struct.unpack_from("<IBBHQQ", data, at)
        if info & 0xF != 2 or not size or not 0 < index < count:  # STT_FUNC, defined
            continue
        section = sections[index]
        end = data.index(b"\0", strings[4] + name)
        found.append((value - section[3] + section[4], size,
                      data[strings[4] + name:end].decode()))
    return sorted(found)


def order(library, traces):
    """The functions of *library* on the pages *traces* read, those read
    by the most traces first, then by where they lie."""
    known = functions(library)
    starts = [offset for offset, _, _ in known]
    uses = Counter()
    for path in traces:
        read = set()
        for line in open(path):
            page = int(line)
            index = max(bisect.bisect_right(starts, page) - 1, 0)
            while index < len(known) and known[index][0] < page + PAGE:
                if known[index][0] + known[index][1] > page:
                    read.add(index)
                index += 1
        uses.update(read)
    return [known[index][2] for index in sorted(uses, key=lambda i: (-uses[i], known[i][0]))]


#: A run that only imports, beside the scripts: every script imports too,
#: so the code of the import is read by one run more than the drawing code
#: all scripts share, and lies first and together rather than among it.
IMPORT_ONLY = "import numpy, PIL.Image, dateutil.parser\nimport pltrs.pyplot\n"


def _scripts():
    """The benchmark scripts with pltrs's imports, and the run that only
    imports (`IMPORT_ONLY`), in ``WORK/scripts``."""
    scripts = WORK / "scripts"
    scripts.mkdir(parents=True, exist_ok=True)
    for path in sorted((ROOT / "benchmarks/python").glob("*.py")):
        text = path.read_text().replace("import matplotlib", "import pltrs")
        (scripts / path.name).write_text(text.replace("from matplotlib", "from pltrs"))
    (scripts / "import_only.py").write_text(IMPORT_ONLY)
    return [path for path in sorted(scripts.glob("*.py")) if path.stem != "_common"]


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--trace", nargs=argparse.REMAINDER, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.trace:
        out, script, *arguments = args.trace
        trace(out, script, arguments)
        return
    runs = [(script, fmt) for script in _scripts()
            for fmt in FORMATS.get(script.stem, ("png",))]
    environment = dict(os.environ, PYTHONPATH=str(ROOT / "python"), PLTRS_LOOK="matplotlib",
                       MPLBACKEND="agg", OPENBLAS_NUM_THREADS="1",
                       PLTRSCONFIGDIR=str(WORK / "config"), PYTHONDONTWRITEBYTECODE="1")

    def run(job):
        script, fmt = job
        out = WORK / f"{script.stem}-{fmt}.pages"
        done = subprocess.run(
            [sys.executable, __file__, "--trace", str(out), str(script),
             str(WORK / f"{script.stem}.{fmt}")],
            env=environment, cwd=WORK, capture_output=True, text=True)
        if done.returncode:
            raise SystemExit(f"{script.name} ({fmt}) failed:\n{done.stderr[-3000:]}")
        return out

    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        traces = list(pool.map(run, runs))
    extension = next((ROOT / "python/pltrs").glob("_native*.so"))
    names = order(extension, traces)
    ORDER.write_text(
        "# The order in which the linker lays out the extension's functions, those\n"
        "# the most programs run first: written by tools/symbol_order.py from the\n"
        f"# code {len(runs)} runs of the benchmark scripts read; see build.rs.\n"
        + "".join(f"{name}\n" for name in names))
    print(f"{ORDER.relative_to(ROOT)}: {len(names)} functions from {len(runs)} runs")


if __name__ == "__main__":
    main()
