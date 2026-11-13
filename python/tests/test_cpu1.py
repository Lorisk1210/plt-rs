"""
numpy runs one of three copies of its loops on x86-64, by the processor:
its baseline (``v2``), the one for AVX2 (``v3``) or the one for AVX-512
(``v4``). Its unstable ``argsort`` leaves equal values in another order in
each, and matplotlib draws equal depths in that order. pltrs computes at
the level of the numpy beside it (`python/DESIGN.md`, "Processor levels").

A machine with AVX-512 is all three: ``NPY_DISABLE_CPU_FEATURES`` holds
numpy to a lower level, which is how the values here were recorded from
matplotlib 3.11.2 with numpy 2.5.4, and how each test runs pltrs at every
level the processor has.
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
#: What ``NPY_DISABLE_CPU_FEATURES`` takes away for numpy to run at a level.
DISABLED = {
    "v2": "X86_V3 X86_V4 AVX512_ICL AVX512_SPR",
    "v3": "X86_V4 AVX512_ICL AVX512_SPR",
    "v4": "",
}
#: Whether this numpy names the groups `DISABLED` takes away (numpy 2.5
#: does; the levels below the processor's can only be run with one that
#: does).
HAS_GROUPS = "X86_V3" in np._core._multiarray_umath.__cpu_features__

#: The levels numpy can be run at here: the processor's and, with a numpy
#: that has the groups, those below.
RUNNABLE = [level for level in LEVELS
            if level == NUMPY_LEVEL or (HAS_GROUPS and level < NUMPY_LEVEL)]

#: A 3-D scatter of 500 points on five places: equal depths, a hundred
#: times each. Prints the order the points are drawn in.
SCATTER = """
import json
import numpy as np
from pltrs.figure import Figure
from pltrs._native import numeric

n = 500
index = np.arange(n)
place = (index * 7) % 5
colors = np.column_stack([(index // 25) / 19, (index % 25) / 24, np.zeros(n), np.ones(n)])
fig = Figure(figsize=(4, 3), dpi=50)
ax = fig.add_subplot(projection="3d")
points = ax.scatter(place * 0.2, place * 0.1, place * 0.3, c=colors, s=30, depthshade=False)
fig.draw_without_rendering()
order = [int(round(r * 19)) * 25 + int(round(g * 24)) for r, g, _, _ in points.get_facecolor()]
print(json.dumps({"level": numeric.cpu_level(), "order": order}))
"""

#: The first points drawn and a digest of the whole order under matplotlib
#: 3.11.2 with numpy 2.5.4 at each level.
MATPLOTLIB_ORDER = {
    "v2": ([0, 85, 350, 75, 345, 340, 80, 335, 330, 325, 295, 320], "383431462889bb57"),
    "v3": ([495, 0, 490, 485, 200, 195, 220, 215, 210, 240, 40, 35], "cfafd1259e5e1cd6"),
    "v4": ([480, 0, 495, 485, 5, 210, 225, 230, 365, 360, 90, 15], "c15d456e07f4bf0d"),
}

#: Arrays sorted by numpy and by pltrs (Rust) in one process. Prints
#: whether every order agrees.
ARGSORTS = """
import json
import numpy as np
from pltrs._native import numeric

def arrays():
    state = np.random.RandomState(20261113)
    for n in [*range(0, 40), 255, 256, 257, 300, 511, 640, 1000, 2600, 20000]:
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

differing = [len(values) for values in arrays()
             if np.argsort(values).tolist() != numeric.argsort(values.tolist(), None)]
print(json.dumps({"level": numeric.cpu_level(), "differing": differing}))
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

#: `DISCRETE` under matplotlib 3.11.2 with numpy 2.5.4 on Linux: with the
#: C library's functions (numpy below AVX-512) and with SVML's.
_LOW = [-16.0, -8.0, -4.0, -2.0, -1.0, -0.5, -0.25, 0.0, 0.5, 1.0, 2.0, 4.0, 8.0]
MATPLOTLIB_DISCRETE = {
    "c library": {"view": [10.0, 100.0], "subs": _LOW,
                  "base2": [-2.0, -1.0, -0.5, -0.125, 0.0, 0.25, 0.5, 1.0],
                  "inverse": "9ed021e71e6d5658", "boundaries": "b2678a4645336129"},
    "svml": {"view": [1.0, 10.0, 100.0], "subs": [*_LOW, 16.0],
             "base2": [-2.0, -1.0, -0.5, -0.125, 0.0, 0.25, 0.5, 1.0, 2.0],
             "inverse": "25df83e3627adff5", "boundaries": "40986dfd99ad244f"},
}


def _at(level, program):
    """What *program* prints last, as JSON, in a fresh interpreter whose
    numpy runs at *level*."""
    environment = {**os.environ, "MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib"}
    if level != NUMPY_LEVEL:
        environment["NPY_DISABLE_CPU_FEATURES"] = DISABLED[level]
    done = subprocess.run([sys.executable, "-c", program], env=environment,
                          capture_output=True, text=True, check=True)
    return json.loads(done.stdout.splitlines()[-1])


def test_pltrs_computes_at_numpys_level():
    features = np._core._multiarray_umath.__cpu_features__
    if HAS_GROUPS:
        expected = "v4" if features["X86_V4"] else "v3" if features["X86_V3"] else "v2"
    else:
        expected = "v4" if features.get("AVX512_SKX") else "v3" if features.get("AVX2") else "v2"
    assert NUMPY_LEVEL == expected
    # The level is resolved once: naming another changes nothing.
    assert numeric.follow_numpy(2, False, False, False, False) == expected


@pytest.mark.parametrize("level", RUNNABLE)
def test_numpy_held_to_a_level_holds_pltrs_to_it(level):
    assert _at(level, "from pltrs._native import numeric\n"
                      "import json\nprint(json.dumps(numeric.cpu_level()))") == level


@pytest.mark.parametrize("level", RUNNABLE)
def test_argsort_is_numpys_at_each_level(level):
    assert _at(level, ARGSORTS) == {"level": level, "differing": []}


@pytest.mark.parametrize("level", RUNNABLE)
def test_equal_depths_are_drawn_in_matplotlibs_order_at_each_level(level):
    drawn = _at(level, SCATTER)
    first, digest = MATPLOTLIB_ORDER[level]
    assert drawn["level"] == level
    assert drawn["order"][:12] == first
    assert hashlib.sha256(json.dumps(drawn["order"]).encode()).hexdigest()[:16] == digest


@pytest.mark.skipif(sys.platform != "linux" or not HAS_GROUPS,
                    reason="recorded with glibc and numpy 2.5's SVML")
@pytest.mark.parametrize("level", RUNNABLE)
def test_discrete_decisions_are_matplotlibs_at_each_level(level):
    computed = _at(level, DISCRETE)
    assert computed.pop("level") == level
    assert computed == MATPLOTLIB_DISCRETE["svml" if level == "v4" else "c library"]


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
    assert numeric.argsort(values, None) == orders[NUMPY_LEVEL]
    with pytest.raises(ValueError, match='unknown processor level "avx2"'):
        numeric.argsort(values, "avx2")
