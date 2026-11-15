#!/usr/bin/env python3
"""
Where matplotlib's builds for arm64 macOS and aarch64 Linux fuse a product
into a sum.

clang contracts ``a * b + c`` into one fused multiply-add where both stand
in one expression, so the extensions of matplotlib's and contourpy's arm64
macOS wheels round such expressions once where the x86-64 wheels round
twice. pltrs computes them the same way on that platform
(``pltrs_core::numeric::fused``; see "Apple silicon" in
``python/DESIGN.md``). This script finds the expressions, for a new
release's wheel to be read again. It runs on an arm64 Mac with Xcode's
command line tools (``otool``, ``clang``, ``llvm-objdump``, ``c++filt``)
and needs nothing but Python's standard library.

  fused_sites.py wheel WHEEL_OR_EXTENSION...

    Says which file it read, with its SHA-256 (a wheel's is the digest
    PyPI lists), and counts the fused instructions (``fmadd``, ``fmsub``,
    ``fnmadd``, ``fnmsub`` and the vector ``fmla``, ``fmls``) of each
    function of each extension. The wheels are linked with link-time
    optimisation: a function's count includes what was inlined into it.

  fused_sites.py sources [-I DIR]... [-D NAME[=VALUE]]... [--std STD] SOURCE...

    Compiles the SOURCEs as the wheels are compiled, plus debug
    information (``clang -arch arm64 -O3 -g``), and prints the function,
    the source line and the expression of every fused instruction, inlined
    code under the line it was inlined from. Lines no code path of
    matplotlib reaches (printing, other dimensions, options it does not
    set) show too: read them against the port.

The sources of matplotlib 3.11.2's wheel, with M its sdist, Q qhull 8.0.2
and H HarfBuzz 14.1.0 (the archives ``subprojects/*.wrap`` name), C
contourpy 1.4.0's sdist, P pybind11's and Y Python's include directories:

  qhull        -I Q/src  Q/src/libqhull_r/*.c
  _tri         --std c++17 -I P -I Y -I M/src  M/src/tri/_tri.cpp
  contourpy    --std c++17 -I P -I Y  C/src/*.cpp
  Agg, _path, _image, _backend_agg
               --std c++17 -I P -I Y -I M/extern/agg24-svn/include -I M/src
               -D MPL_DEVNULL=/dev/null -D MPL_FIX_AGG_IMAGE_FILTER_LUT_BUGS
               -D MPL_FIX_AGG_INTERPOLATION_ENDPOINT_BUG
               M/src/_path_wrapper.cpp M/src/_image_wrapper.cpp
               M/src/_backend_agg.cpp M/src/_backend_agg_wrapper.cpp
               M/src/py_converters.cpp M/extern/agg24-svn/src/*.cpp
  HarfBuzz (in ft2font)
               --std c++17 -I H/src -D HB_NO_MT  H/src/harfbuzz.cc

The rule behind the instructions is syntactic, which makes the operand
order of a port certain: of a sum or difference clang fuses the left
operand if it is a product, otherwise the right one; a product kept in a
variable, converted to another type or computed by an overloaded operator
is rounded first.

  fused_sites.py gimple [-I DIR]... [-D NAME[=VALUE]]... [--std STD] SOURCE...

    The aarch64 Linux wheels are GCC's (GCC 14.2.1 of the
    ``quay.io/pypa/manylinux_2_28_aarch64`` image matplotlib's and
    contourpy's wheels were built in; run it there). GCC fuses across
    statements and inlined operators too, and which product of a sum it
    fuses follows the order its ``widening_mul`` pass meets them, not the
    syntax, so the instructions alone do not say which operand a fused
    product was. This compiles the SOURCEs as the wheels are (``-O3``,
    ``-fvisibility=hidden``; without LTO, which changes no operand of the
    functions pltrs ports)
    with that pass's dump (``-fdump-tree-widening_mul-lineno``) and prints,
    per source line and function, GCC's ``.FMA``/``.FMS``/``.FNMA``/``.FNMS``
    statements with the line's other arithmetic and the debug binds that
    name its variables. ISO C (``--std c11``, numpy's) is not contracted:
    the command says so instead of compiling. Confirm a reading by an exact
    emulation against the wheel on seeded inputs.
"""

import argparse
import collections
import hashlib
import pathlib
import re
import subprocess
import sys
import tempfile
import zipfile

FUSED = re.compile(r"\b(fmadd|fmsub|fnmadd|fnmsub|fmla|fmls)\b")


def run(*command):
    """The standard output of ``command``."""
    return subprocess.run(command, capture_output=True, text=True, check=True).stdout


def demangled(symbols):
    """``symbols`` as C++ writes them, by name."""
    names = sorted(symbols)
    if not names:
        return {}
    plain = subprocess.run(["xcrun", "c++filt"], input="\n".join(names), capture_output=True,
                           text=True, check=True).stdout.splitlines()
    return dict(zip(names, plain))


def instructions(counter):
    return " ".join(f"{count} {mnemonic}" for mnemonic, count in sorted(counter.items()))


def read_extension(path, label):
    """Prints the fused instructions of the extension ``path``, by function."""
    counts = collections.defaultdict(collections.Counter)
    symbol = None
    for row in run("otool", "-tvV", str(path)).splitlines():
        if match := re.match(r"^(\S+):$", row):
            symbol = match.group(1)
        elif match := FUSED.search(row):
            counts[symbol][match.group(1)] += 1
    total = collections.Counter()
    for found in counts.values():
        total.update(found)
    print(f"{label}: {sum(total.values())} fused instructions ({instructions(total) or 'none'})")
    names = demangled(counts)
    for symbol, found in sorted(counts.items(), key=lambda item: -sum(item[1].values())):
        print(f"  {sum(found.values()):4}  {names[symbol]}  [{instructions(found)}]")


def wheel(arguments):
    """The ``wheel`` command."""
    for path in map(pathlib.Path, arguments.files):
        print(f"{path.name}  SHA-256 {hashlib.sha256(path.read_bytes()).hexdigest()}")
        if path.suffix != ".whl":
            read_extension(path, path.name)
            continue
        with zipfile.ZipFile(path) as archive, tempfile.TemporaryDirectory() as scratch:
            for name in sorted(n for n in archive.namelist() if n.endswith(".so")):
                read_extension(archive.extract(name, scratch), name)


def sources(arguments):
    """The ``sources`` command."""
    flags = [f"-I{directory}" for directory in arguments.include]
    flags += [f"-D{definition}" for definition in arguments.define]
    sites = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    with tempfile.TemporaryDirectory() as scratch:
        for index, source in enumerate(map(pathlib.Path, arguments.sources)):
            language = [] if source.suffix == ".c" else [f"-std={arguments.std}"]
            compiled = pathlib.Path(scratch) / f"{index}.o"
            subprocess.run(["clang", "-arch", "arm64", "-O3", "-g", "-DNDEBUG", *language, *flags,
                            "-c", str(source), "-o", str(compiled)], check=True)
            symbol = line = None
            listing = run("xcrun", "llvm-objdump", "-d", "-l", "--no-show-raw-insn", str(compiled))
            for row in listing.splitlines():
                if match := re.match(r"^[0-9a-f]+ <(.+)>:$", row):
                    symbol = match.group(1)
                elif match := re.match(r"^; (.+):(\d+)$", row):
                    line = (match.group(1), int(match.group(2)))
                elif (match := FUSED.search(row)) and line is not None:
                    sites[line][symbol][match.group(1)] += 1
    names = demangled({symbol for functions in sites.values() for symbol in functions})
    texts = {}
    for (path, number), functions in sorted(sites.items()):
        if path not in texts:
            try:
                texts[path] = pathlib.Path(path).read_text(errors="replace").splitlines()
            except OSError:
                texts[path] = []
        expression = texts[path][number - 1].strip() if number <= len(texts[path]) else "?"
        print(f"{path}:{number}: {expression}")
        for symbol, found in sorted(functions.items()):
            print(f"    {names[symbol]}  [{instructions(found)}]")


#: A statement of a ``-lineno`` GIMPLE dump: its source file, line and text.
GIMPLE_STATEMENT = re.compile(r"\s*\[([^\]:]+):(\d+):\d+[^\]]*\] (.*)")

#: GCC's internal functions for the four fused operations.
GIMPLE_FUSED = re.compile(r"= \.(FMA|FMS|FNMA|FNMS) ")

#: Statements worth showing beside a fused one: arithmetic and debug binds.
GIMPLE_CONTEXT = re.compile(r"\.F(MA|MS|NMA|NMS) |[-+*/] |DEBUG [A-Za-z_]")


def gimple_sites(dump):
    """The statements of the dump at ``dump`` on lines holding a fused
    operation: {(file, line): {function: [statement, ...]}}."""
    by_line = collections.defaultdict(lambda: collections.defaultdict(list))
    function = None
    for row in dump.read_text(errors="replace").splitlines():
        if row.startswith(";; Function "):
            function = row.split()[2]
        elif match := GIMPLE_STATEMENT.match(row):
            if GIMPLE_CONTEXT.search(match.group(3)):
                by_line[(match.group(1), int(match.group(2)))][function].append(match.group(3))
    return {line: functions for line, functions in by_line.items()
            if any(GIMPLE_FUSED.search(s) for statements in functions.values() for s in statements)}


def gimple(arguments):
    """The ``gimple`` command."""
    if re.fullmatch(r"(c|iso9899:)\d+", arguments.std):
        print(f"{arguments.std} is ISO C, which GCC does not contract: nothing is fused")
        return
    target = run("gcc", "-dumpmachine").strip()
    if not target.startswith("aarch64"):
        sys.exit(f"gcc targets {target}: run this with the aarch64 GCC that built the wheels")
    flags = [f"-I{directory}" for directory in arguments.include]
    flags += [f"-D{definition}" for definition in arguments.define]
    texts = {}
    with tempfile.TemporaryDirectory() as scratch:
        for index, source in enumerate(map(pathlib.Path, arguments.sources)):
            compiler = ["gcc"] if source.suffix == ".c" else ["g++", f"-std={arguments.std}"]
            out = pathlib.Path(scratch) / str(index)
            out.mkdir()
            # Hidden visibility, as pybind11's build gives the extensions:
            # without it GCC may not inline the operators whose products it
            # fuses across functions.
            subprocess.run([*compiler, "-O3", "-g", "-fPIC", "-fvisibility=hidden", "-DNDEBUG", *flags,
                            "-fdump-tree-widening_mul-lineno", "-c", str(source.resolve()),
                            "-o", "object.o"], cwd=out, check=True)
            for dump in sorted(out.glob("*.widening_mul")):
                for (path, number), functions in sorted(gimple_sites(dump).items()):
                    if path not in texts:
                        try:
                            texts[path] = pathlib.Path(path).read_text(errors="replace").splitlines()
                        except OSError:
                            texts[path] = []
                    lines = texts[path]
                    expression = lines[number - 1].strip() if number <= len(lines) else "?"
                    print(f"{path}:{number}: {expression}")
                    for function, statements in sorted(functions.items()):
                        print(f"    {function}")
                        print("\n".join(f"        {statement}" for statement in statements))


def main():
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    read = commands.add_parser("wheel", help="count the fused instructions of wheels or extensions")
    read.add_argument("files", nargs="+", metavar="WHEEL_OR_EXTENSION")
    read.set_defaults(run=wheel)
    build = commands.add_parser("sources", help="list the source lines compiled to fused instructions")
    build.add_argument("-I", dest="include", action="append", default=[], metavar="DIR")
    build.add_argument("-D", dest="define", action="append", default=[], metavar="NAME[=VALUE]")
    build.add_argument("--std", default="c++17", help="the C++ standard (default: c++17)")
    build.add_argument("sources", nargs="+", metavar="SOURCE")
    build.set_defaults(run=sources)
    dump = commands.add_parser("gimple", help="GCC's fused statements by source line (aarch64 Linux)")
    dump.add_argument("-I", dest="include", action="append", default=[], metavar="DIR")
    dump.add_argument("-D", dest="define", action="append", default=[], metavar="NAME[=VALUE]")
    dump.add_argument("--std", default="c++17", help="the C or C++ standard (default: c++17)")
    dump.add_argument("sources", nargs="+", metavar="SOURCE")
    dump.set_defaults(run=gimple)
    arguments = parser.parse_args()
    if arguments.command != "gimple" and sys.platform != "darwin":
        parser.error("this reads Mach-O extensions with Xcode's tools: run it on a Mac")
    arguments.run(arguments)


if __name__ == "__main__":
    main()
