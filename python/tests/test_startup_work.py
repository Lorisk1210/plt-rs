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


@pytest.mark.skipif(sys.platform != "linux" or platform.machine() != "x86_64",
                    reason="the order is given to the linker on x86_64 Linux")
def test_extension_lays_out_functions_in_order():
    tool = runpy.run_path(str(ROOT / "crates/pltrs-python/tools/symbol_order.py"))
    offsets = {name: offset for offset, _, name in tool["functions"](pltrs._native.__file__)}
    named = [line for line in tool["ORDER"].read_text().splitlines()
             if line and not line.startswith("#")]
    laid_out = [offsets[name] for name in named[:500] if name in offsets]
    if len(laid_out) < 250:
        # cargo names a crate's functions after the profile it builds:
        # the order is the extension profile's (`cargo xtask python
        # --release`, the wheels), and other builds keep the compiler's.
        pytest.skip("the extension was built with another profile than the order's")
    assert laid_out == sorted(laid_out)
