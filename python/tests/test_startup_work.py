"""
What ``import pltrs.pyplot`` does quicker than matplotlib's code would,
with the same results: docstrings cleaned without going through their
lines, the ``ClassName:kwdoc`` tables kept between runs, `Artist.set`'s
signature and docstring made when first looked up, the built-in colormaps'
segments as matplotlib keeps them, and the extension's functions laid out
in the order the benchmark scripts run them. Expected values are matplotlib
3.11.2's.
"""

import inspect
import json
import os
import pathlib
import platform
import random
import re
import runpy
import subprocess
import sys

import pytest

import pltrs
from pltrs import _artist_inspector, _docstring, artist, lines

ROOT = pathlib.Path(__file__).resolve().parents[2]


def _docstrings():
    """The docstrings of the package's loaded modules, classes and functions."""
    for name, module in list(sys.modules.items()):
        if not name.startswith(pltrs.__name__) or module is None:
            continue
        for value in [module, *vars(module).values()]:
            doc = getattr(value, "__doc__", None)
            if isinstance(doc, str):
                yield doc
            for member in vars(value).values() if isinstance(value, type) else ():
                doc = getattr(member, "__doc__", None)
                if isinstance(doc, str):
                    yield doc


def test_cleandoc_is_inspects():
    import pltrs.pyplot  # noqa: F401  (every docstring import cleans)
    edges = ["", "\n", "  ", "a", " a", "\n\n a\n b\n", "a\n\tb\n  c", "  a\n    b\n  c\n   ",
             "a\n\n  \nb\n", "\r\n a\r\n b", "a\n b\n\n\n", "\n\n\n", "x\n y\n  z\n \n"]
    for doc in [*edges, *_docstrings()]:
        assert _docstring.cleandoc(doc) == inspect.cleandoc(doc)


def test_parameter_types_as_matplotlibs_expression():
    rng = random.Random(5)
    pieces = [" ", "*", "x", "y", " : ", ":", "\n", "\r", "a", "x : ", "*x : ", "  "]
    for _ in range(20000):
        text = "".join(rng.choice(pieces) for _ in range(rng.randrange(14)))
        for name in ("x", "xy"):
            match = re.search(fr"(?m)^ *\*?{name} : (.+)", text)
            assert _artist_inspector._parameter_type(text, name) == (
                match.group(1) if match else None)


def test_valid_values_as_matplotlibs():
    inspector = artist.ArtistInspector(lines.Line2D)
    assert inspector.get_valid_values("linestyle") == (
        "{'-', '--', '-.', ':', '', ...} or (offset, on-off-seq)")
    assert inspector.get_valid_values("drawstyle") == (
        "{'default', 'steps', 'steps-pre', 'steps-mid', 'steps-post'}, default: 'default'")
    assert inspector.get_valid_values("markevery") == (
        "None or int or (int, int) or slice or list[int] or float or (float, float) "
        "or list[bool]")
    assert inspector.get_valid_values("alpha") == "float or None"


# A fresh interpreter importing pyplot, then printing whether the kwdoc
# file was there before and a digest of docstrings holding kwdoc tables.
DOCSTRINGS = """
import glob, hashlib, os, sys
before = glob.glob(os.path.join(sys.argv[1], "kwdoc-*.pltrs"))
import pltrs.pyplot as plt
from pltrs.figure import Figure
from pltrs.lines import Line2D
from pltrs.patches import Rectangle
docs = [plt.Axes.plot.__doc__, Line2D.__init__.__doc__, Figure.add_axes.__doc__,
        Rectangle.__init__.__doc__, plt.plot.__doc__, plt.bar.__doc__]
print(bool(before), hashlib.sha256("".join(docs).encode()).hexdigest())
"""


def _docstrings_run(config):
    env = dict(os.environ, PLTRSCONFIGDIR=str(config), MPLBACKEND="agg")
    env.pop("MPLCONFIGDIR", None)
    result = subprocess.run([sys.executable, "-c", DOCSTRINGS, str(config)], env=env,
                            capture_output=True, text=True, check=True)
    found, digest = result.stdout.split()
    return found == "True", digest


def test_kwdoc_tables_kept_between_runs(tmp_path):
    first = _docstrings_run(tmp_path)
    second = _docstrings_run(tmp_path)
    assert (first[0], second[0]) == (False, True)
    assert first[1] == second[1]
    (kept,) = tmp_path.glob("kwdoc-*.pltrs")
    kept.write_bytes(b"not a table")
    assert _docstrings_run(tmp_path)[1] == first[1]
    assert kept.read_bytes() != b"not a table"


def test_kept_tables_are_those_the_docstrings_hold():
    # Computed as the class was defined, as matplotlib's are: a setter without
    # a docstring of its own reads "unknown" then (``transform``).
    table = _docstring._kwdoc_tables.get(lines.Line2D, artist.kwdoc)
    assert table == _docstring.interpd.params["Line2D:kwdoc"]
    assert table in lines.Line2D.__init__.__doc__
    assert "\n    transform: unknown\n" in table


def test_artist_set_as_matplotlibs():
    signature = inspect.signature(artist.Artist.set)
    assert list(signature.parameters)[:8] == [
        "self", "agg_filter", "alpha", "animated", "clip_box", "clip_on", "clip_path", "gid"]
    assert len(signature.parameters) == 20
    assert artist.Artist.set.__doc__.splitlines()[-3:] == [
        "    url: str", "    visible: bool", "    zorder: float"]
    made = vars(artist.Artist)["set"]
    assert inspect.isfunction(made) and made._autogenerated_signature


def test_builtin_segments_as_matplotlibs():
    assert pltrs.colormaps["jet"]._segmentdata["red"] == (
        (0.0, 0, 0), (0.35, 0, 0), (0.66, 1, 1), (0.89, 1, 1), (1.0, 0.5, 0.5))
    assert pltrs.colormaps["jet_r"]._segmentdata["red"] == [
        (0.0, 0.5, 0.5), (0.10999999999999999, 1, 1), (0.33999999999999997, 1, 1),
        (0.65, 0, 0), (1.0, 0, 0)]


#: How to write the order again (see the tool's docstring).
_REORDER = ("run `.venv-pltrs/bin/python crates/pltrs-python/tools/symbol_order.py` twice, "
            "with `cargo xtask python --release` before each run")


def _skip_unless_built_with_the_orders_profile():
    """Skips where the extension was not built with the profile the order
    names the functions of."""
    profile = pltrs._native.diagnostics.build_profile()
    if profile != "extension":
        # cargo names a crate's functions after the profile it builds:
        # the order is the extension profile's (`cargo xtask python
        # --release`, the wheels), and other builds keep the compiler's.
        pytest.skip(f"the extension was built with the {profile!r} profile, "
                    "the order is the 'extension' profile's")


def _ordered_offsets():
    """The order file's function names, and where the extension built lays
    out each it has; skips where the extension was not built with the
    order's profile."""
    _skip_unless_built_with_the_orders_profile()
    tool = runpy.run_path(str(ROOT / "crates/pltrs-python/tools/symbol_order.py"))
    if not tool["has_symbol_table"](pltrs._native.__file__):
        # The wheels' extension is stripped (Zig links it so): its order
        # is checked on an unstripped link of the same build.
        pytest.skip("the extension has no symbol table to read the order from")
    offsets = {name: offset for offset, _, name in tool["functions"](pltrs._native.__file__)}
    named = [line for line in tool["ORDER"].read_text().splitlines()
             if line and not line.startswith("#")]
    return named, offsets


_ON_ORDERED_TARGET = pytest.mark.skipif(
    sys.platform != "linux" or platform.machine() != "x86_64",
    reason="the order is given to the linker on x86_64 Linux")


@_ON_ORDERED_TARGET
def test_extension_lays_out_functions_in_order():
    named, offsets = _ordered_offsets()
    laid_out = [offsets[name] for name in named[:500] if name in offsets]
    assert laid_out == sorted(laid_out)
    # A cheap second line: most of the functions the file names still
    # exist (all of them when it is written; 95.6% after the 80 commits
    # that let it go stale before task PERF9). A change to how cargo builds
    # a crate renames all its functions and those of every crate above it:
    # a build script given to pltrs-core (92663e93) left 5.5% of them, the
    # linker ordered nothing, and the extension a small figure holds grew
    # from 6,484 to 14,100 KiB while this test, telling the profile by the
    # names, took it for another profile's build and skipped.
    present = sum(name in offsets for name in named) / len(named)
    assert present >= 0.97, f"{present:.1%} of the order's functions exist: {_REORDER}"


#: The most of the extension's code a process may hold, in KiB, after
#: ``import pltrs.pyplot`` and after saving a small figure as PNG, with the
#: kernel mapping 64 KiB around each page it faults in: a quarter above
#: what the order of task PERF9 (2026-11-14) gives when it is written,
#: 3,808 and 5,664 KiB. The code the program runs, laid out together, is
#: what keeps them low; with the order 80 commits stale they were 5,344
#: and 8,800 KiB, as the functions added since lay among those no program
#: runs. The room is for the changes between two writings: the first
#: change of native code after PERF9's order took the figure to 6,132 KiB
#: (functions the order names no longer exist under that name once the
#: code around them changes). The order is written again before a release
#: and an official benchmark (RELEASING.md), and whenever this fails.
RESIDENT_CODE_CEILING_KIB = {"import": 4_750, "figure": 7_100}

#: Measures, in a fresh process, the fault-around window (the pages that
#: become present when one page of a cached file is read through a
#: mapping) and the extension's code resident after the import and after a
#: small figure, the whole extension read first so that the page cache
#: holds it as after a build or an install.
_RESIDENT_PROGRAM = """
import io, json, mmap, os, sys
PAGE = os.sysconf("SC_PAGE_SIZE")


def present(path, executable):
    total, current = 0, None
    with open("/proc/self/smaps") as smaps:
        for line in smaps:
            fields = line.split()
            if "-" in fields[0] and len(fields) >= 5 and not fields[0].endswith(":"):
                current = (fields[1], fields[5] if len(fields) >= 6 else "")
            elif fields[0] == "Rss:" and current and current[1] == path and (
                    not executable or "x" in current[0]):
                total += int(fields[1])
    return total


probe = os.path.join(sys.argv[1], "fault-around")
with open(probe, "wb") as file:
    file.write(bytes(64 * PAGE))
with open(probe, "rb") as file:
    file.read()
    mapped = mmap.mmap(file.fileno(), 64 * PAGE, prot=mmap.PROT_READ)
    mapped[32 * PAGE]
    window_kib = present(probe, False)
    mapped.close()
import numpy, PIL.Image, dateutil.parser
import pltrs
native = os.path.realpath(pltrs._native.__file__)
with open(native, "rb") as file:
    while file.read(1 << 24):
        pass
import pltrs.pyplot as plt
at_import = present(native, True)
fig, ax = plt.subplots()
ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])
ax.set_title("A title")
ax.set_xlabel("x")
fig.savefig(io.BytesIO(), format="png")
print(json.dumps({"window_kib": window_kib, "page": PAGE, "import": at_import,
                  "figure": present(native, True)}))
"""


@_ON_ORDERED_TARGET
def test_extension_code_a_small_program_holds_stays_under_its_ceiling(tmp_path):
    # Stripped libraries too (the wheels Zig links): what they hold needs
    # no symbol table, and it is how an order whose names the wheel's build
    # does not have shows there (task REL2: every wheel from 92663e93 on
    # held 8,308 KiB at import and 14,132 after a figure).
    _skip_unless_built_with_the_orders_profile()
    env = {**os.environ, "MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib",
           "PLTRSCONFIGDIR": str(tmp_path)}

    def measure():
        result = subprocess.run([sys.executable, "-c", _RESIDENT_PROGRAM, str(tmp_path)],
                                env=env, capture_output=True, text=True, check=True)
        return json.loads(result.stdout.splitlines()[-1])

    measure()  # the caches of a first import written
    resident = measure()
    if resident["page"] != 4096 or resident["window_kib"] != 64:
        pytest.skip(f"the kernel maps {resident['window_kib']} KiB in pages of "
                    f"{resident['page']} bytes around a fault, not the 64 KiB in pages of "
                    "4096 the ceilings are for")
    over = {what: (resident[what], ceiling)
            for what, ceiling in RESIDENT_CODE_CEILING_KIB.items() if resident[what] > ceiling}
    assert not over, (
        f"KiB of the extension's code resident (held, ceiling): {over}: the code an "
        f"ordinary program runs no longer lies together; {_REORDER}")
