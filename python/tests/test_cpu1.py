"""
numpy runs one of three copies of its loops on x86-64, by the processor:
its baseline (``v2``), the one for AVX2 (``v3``) or the one for AVX-512
(``v4``). Its unstable ``argsort`` leaves equal values in another order in
each, and in another again from release to release in arrays of more than
64 values (numpy 1, 2.0 to 2.2, 2.3 and later), and matplotlib draws equal
depths in that order. pltrs computes at the level of the numpy beside it,
has that numpy sort what is longer than 64 values and replays its sort of
what is shorter (`python/DESIGN.md`, "Processor levels").

A machine with AVX-512 is all three levels: ``NPY_DISABLE_CPU_FEATURES``
holds numpy to a lower one, which is how the values here were recorded
from matplotlib 3.11.2 with numpy 1.26.4, 2.2.6 and 2.5.4, and how each
test runs pltrs at every level the processor has.
"""

import hashlib
import json
import os
import subprocess
import sys

import numpy as np
import pytest

from pltrs._native import numeric

LEVELS = ("v2", "v3", "v4")
#: The level numpy runs at in this process.
NUMPY_LEVEL = numeric.cpu_level()
#: numpy's extension module, which says what it dispatches to.
_UMATH = (np._core if hasattr(np, "_core") else np.core)._multiarray_umath
#: Whether this numpy dispatches by the groups ``X86_V3`` and ``X86_V4``
#: (2.4 and later) rather than by single features.
HAS_GROUPS = "X86_V3" in _UMATH.__cpu_features__
#: The numpy releases whose sort is one edition, by the first of them.
SORT_EDITION = ("numpy 1.25" if np.lib.NumpyVersion(np.__version__) < "2.0.0" else
                "numpy 2.0" if np.lib.NumpyVersion(np.__version__) < "2.3.0" else "numpy 2.3")


def _disabled(level):
    """What ``NPY_DISABLE_CPU_FEATURES`` takes away for this numpy to run
    at *level*: groups, or for a numpy without them every AVX-512 feature
    it dispatches to and, for ``v2``, AVX2 and FMA as well."""
    if HAS_GROUPS:
        return {"v2": "X86_V3 X86_V4 AVX512_ICL AVX512_SPR",
                "v3": "X86_V4 AVX512_ICL AVX512_SPR", "v4": ""}[level]
    wide = [name for name in _UMATH.__cpu_dispatch__ if name.startswith("AVX512")]
    return " ".join({"v4": [], "v3": wide, "v2": ["FMA3", "AVX2", *wide]}[level])


#: The levels numpy can be run at here: the processor's and those below.
RUNNABLE = [level for level in LEVELS if level <= NUMPY_LEVEL]

#: 3-D scatters of points on a diagonal, with equal depths: 500 points on
#: five places, 150 on five places (where numpy 1 already partitions and
#: later ones sort by networks), and 300 in depth order, three to a place
#: (which numpy 2.3 and later leave alone). Prints a digest of the order
#: the points of each are drawn in.
SCATTERS = """
import hashlib
import json
import numpy as np
from pltrs.figure import Figure
from pltrs._native import numeric


def drawn(n, place):
    index = np.arange(n)
    at = place(index)
    colors = np.column_stack([(index // 25) / 19, (index % 25) / 24, np.zeros(n), np.ones(n)])
    fig = Figure(figsize=(4, 3), dpi=50)
    ax = fig.add_subplot(projection="3d")
    points = ax.scatter(at * 0.2, at * 0.1, at * 0.3, c=colors, s=30, depthshade=False)
    fig.draw_without_rendering()
    order = [int(round(r * 19)) * 25 + int(round(g * 24)) for r, g, _, _ in points.get_facecolor()]
    assert sorted(order) == list(range(n))
    return hashlib.sha256(json.dumps(order).encode()).hexdigest()[:16]


print(json.dumps({
    "level": numeric.cpu_level(),
    "orders": [drawn(500, lambda index: (index * 7) % 5),
               drawn(150, lambda index: (index * 7) % 5),
               drawn(300, lambda index: (299 - index) // 3)],
}))
"""

#: The digests `SCATTERS` prints under matplotlib 3.11.2, by the edition
#: of numpy's sort (numpy 1.26.4, 2.2.6 and 2.5.4; 2.0.2 and 2.3.5 give
#: their edition's) and the level. Without AVX2 every numpy sorts with its
#: own code, and numpy 1 has no AVX2 sort.
_OWN_SORT = ["383431462889bb57", "7594b29350dfb0d0", "1797a79fca0df052"]
MATPLOTLIB_ORDERS = {
    "numpy 1.25": {
        "v2": _OWN_SORT, "v3": _OWN_SORT,
        "v4": ["25037af1c024ea59", "ae9356a01df6989b", "8f0defe9410376b1"],
    },
    "numpy 2.0": {
        "v2": _OWN_SORT,
        "v3": ["cfafd1259e5e1cd6", "1f2aa902cb4ff22d", "ff8e87ff3b01c221"],
        "v4": ["c15d456e07f4bf0d", "1f2aa902cb4ff22d", "6ad880dea4b44431"],
    },
    "numpy 2.3": {
        "v2": _OWN_SORT,
        "v3": ["cfafd1259e5e1cd6", "1f2aa902cb4ff22d", "8cf473afecf1704d"],
        "v4": ["c15d456e07f4bf0d", "1f2aa902cb4ff22d", "8cf473afecf1704d"],
    },
}

#: Arrays sorted by numpy and by pltrs (Rust) in one process. Prints the
#: lengths of those the two order otherwise, and how pltrs came to sort
#: arrays without NaN and arrays with.
ARGSORTS = """
import json
import numpy as np
from pltrs._native import numeric

def arrays():
    state = np.random.RandomState(20261113)
    for n in [*range(0, 40), 64, 65, 100, 200, 255, 256, 257, 300, 511, 640, 1000, 2600, 20000]:
        ties = state.randint(0, 7, n) / 4
        yield ties
        with_nan = ties.copy()
        with_nan[::5] = np.nan
        yield with_nan
        infinite = ties.copy()
        infinite[::3] = np.inf
        infinite[1::7] = -np.inf
        yield infinite
        yield np.where(state.randint(0, 2, n) == 1, 0.0, -0.0)
        yield np.sort(ties)
        yield np.sort(ties)[::-1].copy()
        yield state.uniform(size=n)

before = numeric.argsort_routes()
differing = [len(values) for values in arrays()
             if np.argsort(values).tolist() != numeric.argsort(values.tolist(), None)]
print(json.dumps({"level": numeric.cpu_level(), "differing": differing,
                  "before": before, "routes": numeric.argsort_routes()}))
"""

#: An ``argsort`` that is none pltrs replays (a stable one, registered in
#: the place of numpy's method), which counts its calls: pltrs sorts with
#: it what is longer than it replays, and what is shorter too once such a
#: sort came out with equal values. Prints what was sorted how, whether a
#: scatter of equal depths is drawn in that sort's order, and how pltrs
#: came to sort.
ANOTHER_SORT = """
import json
import numpy as np
from pltrs.figure import Figure
from pltrs._native import numeric, register

calls = []


def stable(values):
    calls.append(len(values))
    return np.ndarray.argsort(values, kind="stable")


register("argsort", stable)
nan = float("nan")
distinct = numeric.argsort([3.0, 1.0, 2.0], None)
after_distinct = (list(calls), numeric.argsort_routes())
depths = [float((i * 7) % 5) for i in range(300)]
long = numeric.argsort(depths, None)
after_long = (list(calls), numeric.argsort_routes())
short = [float(i % 3) for i in range(12)]
short_nan = [nan if i % 4 == 0 else float(i % 3) for i in range(60)]
n = 300
index = np.arange(n)
at = (index * 7) % 5
colors = np.column_stack([(index // 25) / 19, (index % 25) / 24, np.zeros(n), np.ones(n)])
fig = Figure(figsize=(4, 3), dpi=50)
points = fig.add_subplot(projection="3d").scatter(at * 0.2, at * 0.1, at * 0.3, c=colors, s=30,
                                                   depthshade=False)
fig.draw_without_rendering()
drawn = [int(round(r * 19)) * 25 + int(round(g * 24)) for r, g, _, _ in points.get_facecolor()]
print(json.dumps({
    "distinct": distinct, "after distinct": after_distinct,
    "long": long == sorted(range(300), key=depths.__getitem__), "after long": after_long,
    "short": numeric.argsort(short, None) == sorted(range(12), key=short.__getitem__),
    "short with nan": numeric.argsort(short_nan, None) == sorted(
        range(60), key=lambda i: (i % 4 == 0, 0 if i % 4 == 0 else i % 3)),
    "drawn by place": [[i for i in drawn if at[i] == place] for place in range(5)],
    "routes": numeric.argsort_routes(),
}))
"""

#: The places where the last bit of numpy's ``sinh``, ``log`` and ``exp``
#: decides something discrete: the powers an `AsinhLocator` rounds its
#: ticks to, and the boundaries a colorbar draws for a `LogNorm`. numpy
#: computes them with Intel's SVML on Linux with AVX-512 and with the C
#: library elsewhere; pltrs takes numpy's own values there.
DISCRETE = """
import hashlib
import json
import numpy as np
from pltrs import colors, ticker
from pltrs.figure import Figure
from pltrs._native import numeric

locator = ticker.AsinhLocator(linear_width=10.0, numticks=7)
locator.create_dummy_axis()
locator.axis.set_view_interval(10.0, 100.0)
norm = colors.LogNorm(0.37, 913.0)
fig = Figure(figsize=(5, 4))
x, y = np.meshgrid(np.linspace(0, 1, 30), np.linspace(0, 1, 30))
mesh = fig.add_subplot().pcolormesh(x, y, 10 ** (3 * x + y), norm=colors.LogNorm(), shading="auto")
bar = fig.colorbar(mesh)
fig.draw_without_rendering()

def digest(values):
    return hashlib.sha256(json.dumps([float(v).hex() for v in values]).encode()).hexdigest()[:16]

print(json.dumps({
    "level": numeric.cpu_level(),
    "view": [float(v) for v in locator()],
    "subs": [float(v) for v in ticker.AsinhLocator(1.0, base=2, subs=(2, 4, 8)).tick_values(-3.0, 2.0)],
    "base2": [float(v) for v in ticker.AsinhLocator(1.0, base=2).tick_values(-3.0, 2.0)],
    "inverse": digest(norm.inverse(np.linspace(0, 1, 257))),
    "boundaries": digest(bar._boundaries),
}))
"""

#: `DISCRETE` under matplotlib 3.11.2 on Linux: with the C library's
#: functions (every numpy below AVX-512), with SVML's (numpy 2.2.6 and
#: 2.5.4 with AVX-512) and with the AVX-512 loops numpy 1.26.4 has of its
#: own.
_LOW = [-16.0, -8.0, -4.0, -2.0, -1.0, -0.5, -0.25, 0.0, 0.5, 1.0, 2.0, 4.0, 8.0]
MATPLOTLIB_DISCRETE = {
    "c library": {"view": [10.0, 100.0], "subs": _LOW,
                  "base2": [-2.0, -1.0, -0.5, -0.125, 0.0, 0.25, 0.5, 1.0],
                  "inverse": "9ed021e71e6d5658", "boundaries": "b2678a4645336129"},
    "svml": {"view": [1.0, 10.0, 100.0], "subs": [*_LOW, 16.0],
             "base2": [-2.0, -1.0, -0.5, -0.125, 0.0, 0.25, 0.5, 1.0, 2.0],
             "inverse": "25df83e3627adff5", "boundaries": "40986dfd99ad244f"},
    "numpy 1 with avx-512": {
        "view": [1.0, 10.0], "subs": [*_LOW, 16.0],
        "base2": [-2.0, -1.0, -0.5, -0.125, 0.0, 0.25, 0.5, 1.0, 2.0],
        "inverse": "8daa351f607beb68", "boundaries": "5a472ad9b87d4a99"},
}


def _at(level, program):
    """What *program* prints last, as JSON, in a fresh interpreter whose
    numpy runs at *level*."""
    environment = {**os.environ, "MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib"}
    if level != NUMPY_LEVEL:
        environment["NPY_DISABLE_CPU_FEATURES"] = _disabled(level)
    done = subprocess.run([sys.executable, "-W", "ignore", "-c", program], env=environment,
                          capture_output=True, text=True, check=True)
    return json.loads(done.stdout.splitlines()[-1])


def _routed(route, level):
    """Whether *route* is how pltrs comes to sort short arrays beside this
    numpy at *level*: by replaying numpy's sort at that level (numpy's
    own introsort where it has no SIMD sort: without AVX2, and numpy 1
    with AVX2), or, off Linux, by numpy itself where its build sorts
    otherwise (another C++ library's ``std::sort``)."""
    replayed = "v2" if (level, SORT_EDITION) == ("v3", "numpy 1.25") else level
    return route == replayed if sys.platform == "linux" else route in (replayed, "numpy")


def test_pltrs_computes_at_numpys_level():
    # The level numpy runs at: the highest whose copy of its loops the
    # processor has and this numpy holds (its wheel for Windows holds none
    # for X86_V4, whatever the processor).
    copies = {*_UMATH.__cpu_baseline__, *_UMATH.__cpu_dispatch__}
    runs = {name: name in copies and enabled for name, enabled in _UMATH.__cpu_features__.items()}
    if HAS_GROUPS:
        expected = "v4" if runs["X86_V3"] and runs["X86_V4"] else "v3" if runs["X86_V3"] else "v2"
    else:
        expected = "v4" if runs.get("AVX512_SKX") else "v3" if runs.get("AVX2") else "v2"
    assert NUMPY_LEVEL == expected
    # The level is resolved once: naming another changes nothing.
    assert numeric.follow_numpy(False, False, False, False) == expected


@pytest.mark.parametrize("level", RUNNABLE)
def test_numpy_held_to_a_level_holds_pltrs_to_it(level):
    assert _at(level, "from pltrs._native import numeric\n"
                      "import json\nprint(json.dumps(numeric.cpu_level()))") == level


@pytest.mark.parametrize("level", RUNNABLE)
def test_argsort_is_numpys_at_each_level(level):
    sorted_ = _at(level, ARGSORTS)
    assert (sorted_["level"], sorted_["differing"]) == (level, [])
    # numpy is asked nothing about short arrays until one comes out with
    # equal values, and pltrs (Rust) replays this numpy's sort of them.
    assert sorted_["before"] == [None, None]
    assert all(_routed(route, level) for route in sorted_["routes"])


@pytest.mark.parametrize("level", RUNNABLE)
def test_equal_depths_are_drawn_in_matplotlibs_order_at_each_level(level):
    drawn = _at(level, SCATTERS)
    assert drawn["level"] == level
    assert drawn["orders"] == MATPLOTLIB_ORDERS[SORT_EDITION][level]


def test_long_arrays_are_sorted_by_numpy_and_short_ones_by_what_sorts_as_it_does():
    sorted_ = _at(NUMPY_LEVEL, ANOTHER_SORT)
    # One order only for distinct values: replayed, nothing asked.
    assert sorted_["distinct"] == [1, 2, 0]
    assert sorted_["after distinct"] == [[], [None, None]]
    # More than 64 values: the registered sort's, whatever it is.
    assert sorted_["long"] and sorted_["after long"] == [[300], [None, None]]
    # Short arrays with equal values: asked about once, and sorted by the
    # registered sort too, since it sorts as nothing pltrs replays.
    assert sorted_["short"] and sorted_["short with nan"]
    assert sorted_["routes"] == ["numpy", "numpy"]
    # matplotlib draws the reverse of the sorted order: the points of one
    # place, of equal depth, from the last to the first.
    for place in sorted_["drawn by place"]:
        assert place == sorted(place, reverse=True) and len(place) == 60


@pytest.mark.skipif(sys.platform != "linux", reason="recorded with glibc and numpy's SVML")
@pytest.mark.parametrize("level", RUNNABLE)
def test_discrete_decisions_are_matplotlibs_at_each_level(level):
    computed = _at(level, DISCRETE)
    assert computed.pop("level") == level
    maths = ("c library" if level != "v4" else
             "numpy 1 with avx-512" if SORT_EDITION == "numpy 1.25" else "svml")
    assert computed == MATPLOTLIB_DISCRETE[maths]


def test_pltrs_replays_every_level_whatever_the_processor():
    # numpy 2.5.4's argsort of these values at each level; they part above
    # 256 values, where x86-simd-sort partitions.
    values = [float((i * 37) % 11) for i in range(300)]
    orders = {level: numeric.argsort(values, level) for level in LEVELS}
    assert [orders[level][:8] for level in LEVELS] == [
        [0, 220, 209, 33, 198, 187, 176, 44],
        [0, 33, 275, 44, 55, 66, 88, 77],
        [0, 297, 44, 55, 66, 88, 253, 242],
    ]
    assert [hashlib.sha256(json.dumps(orders[level]).encode()).hexdigest()[:16]
            for level in LEVELS] == ["7a8e373676e79450", "87a60f7ef47e6c45", "d3db51d96c5138ef"]
    with pytest.raises(ValueError, match='unknown processor level "avx2"'):
        numeric.argsort(values, "avx2")
