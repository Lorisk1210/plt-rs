"""
Runs the sweep's scenes and compares their rows with matplotlib's.

Each scene is a function of a `describe.Rows` that makes its figures and
records its rows; `run` runs every scene (or those named) under
matplotlib's defaults and closes its figures after it.
"""

import json
import pathlib
import platform
import sys

import pltrs
import pltrs.pyplot as plt

from . import (computed, events, formatting, given, handed, plotted, structures, subclassed,
               swept)
from .describe import Rows, kind_of

#: matplotlib's rows, recorded by ``python -m number_types --record``.
RECORDED = pathlib.Path(__file__).with_name("reference.json")

#: The rows matplotlib itself describes otherwise on a platform than in the
#: record, which was made on Linux x86-64, as the sweep run there under
#: matplotlib gave them: ``{platform: {row: description}}``, the platform
#: being ``sys.platform`` or ``"<sys.platform>-<machine>"``. With Apple's
#: libm, and with the products matplotlib's GCC-built aarch64 Linux wheel
#: fuses (recorded in the manylinux_2_28 image, task REL4), the arrow's
#: outline ends a last bit beside its first vertex, so that ``get_verts()``
#: closes it with one vertex more. With numpy's Windows wheel the cross
#: spectral density of integers ends a value a last bit higher (matplotlib
#: 3.11.2 there, measured on the Windows PC, task WIN6), and pltrs gives
#: matplotlib's value there. On aarch64 Linux the same row,
#: ``R csd ints | returns``, keeps x86-64's value, which pltrs gives there
#: too: matplotlib's own value is one ulp off in one FFT value
#: (24.730162019507027 for 24.73016201950703), as ``DIFFERENCES.md``
#: ("Numbers on other machines") declares.
ON_PLATFORM = {
    "darwin": {"S FancyArrowPatch | get_verts()":
               "ndarray[float64,(12, 2)]=[87.5001, 65.0665, 93.3051]"},
    "linux-aarch64": {"S FancyArrowPatch | get_verts()":
                      "ndarray[float64,(12, 2)]=[87.5001, 65.0665, 93.3051]"},
    "win32": {"R csd ints | returns":
              "tuple[2](ndarray[complex128,(9,)]=['(70.57846914232039+0', "
              "'(138.34632891584198-', '(24.73016201950703-4'], "
              "ndarray[float64,(9,)]=[0.0, 0.125, 0.25])"},
}

#: The rows on which pltrs is known to part from matplotlib, with
#: matplotlib's description, pltrs' and why: what is left, written by
#: ``python -m number_types --expect`` and held by
#: ``python/tests/test_num1.py``.
EXPECTED = pathlib.Path(__file__).with_name("expected.json")

#: Why rows may differ, by a part of their name: ``--expect`` writes no
#: row without one.
REASONS = {
    "array layout": "contiguity: matplotlib hands column views (strides) where "
                    "pltrs hands contiguous copies; writability is the same",
    "sticky_edges": "the lists of sticky edges are pltrs' list subclass, which "
                    "tells pltrs (Rust) of each change in place before it "
                    "autoscales: no sync point precedes every native autoscale",
    "draw_path tpath.vertices, affine matrix, rgbFace": "a path effect of the "
                    "program's gets the fill as pltrs (Rust) draws it (four "
                    "floats), not the patch's colour as given",
    "C transform of points | transform_non_affine argument types": "how often "
                    "and in which pieces a program's transform is called "
                    "(GEN11's): matplotlib maps a scatter's masked offsets and a "
                    "text in other calls",
    "clabel": "a manual or inline label's rotation, a value (COLL1's)",
}

#: Every scene, by name.
SCENES = {f"{module.__name__.rsplit('.', 1)[-1]}.{scene.__name__.strip('_')}": scene
          for module in (given, swept, plotted, computed, formatting, structures, handed,
                         subclassed, events)
          for scene in module.SCENES}


def recorded():
    """matplotlib's rows on this platform: ``{row: description}``."""
    return {**json.loads(RECORDED.read_text("utf-8")), **ON_PLATFORM.get(sys.platform, {}),
            **ON_PLATFORM.get(f"{sys.platform}-{platform.machine()}", {})}


def expected():
    """The rows pltrs is known to differ on: ``{row: {"matplotlib": ...,
    "pltrs": ..., "reason": ...}}``."""
    return json.loads(EXPECTED.read_text("utf-8"))


def reason_for(row):
    """Why *row* may differ (`REASONS`), or None."""
    return next((reason for part, reason in REASONS.items() if part in row), None)


def run(names=None):
    """The rows of the scenes *names* (every scene without them), each run
    under matplotlib's defaults: ``{row: description}``."""
    rows = Rows()
    for name, scene in SCENES.items():
        if names and name not in names:
            continue
        with pltrs.rc_context():
            pltrs.rcdefaults()
            try:
                scene(rows)
            except Exception as error:  # noqa: BLE001 - a scene that fails is a row
                rows.found[f"{name} | scene"] = f"raises {type(error).__name__}: {error}"
            finally:
                plt.close("all")
    return rows.found


def differing(ours, theirs):
    """The rows of *theirs* whose description *ours* does not match:
    ``{row: [matplotlib's, pltrs']}``."""
    return {row: [description, ours.get(row)] for row, description in theirs.items()
            if ours.get(row) != description}


def by_kind(differences):
    """The rows of *differences* by the kind of difference: ``{kind: [row]}``."""
    kinds = {}
    for row, (theirs, ours) in differences.items():
        kinds.setdefault(kind_of(theirs, ours), []).append(row)
    return kinds
