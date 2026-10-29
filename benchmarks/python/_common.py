"""What the benchmark scripts share: their data and how they end.

The data comes from the seeded stream plt-rs' workloads use
(``spec_support.SpecRng``, SplitMix64), drawn with numpy in bulk so that
making ten million samples stays quick; each script draws the same figure
from the same numbers as the plt-rs workload of the same name in
``xtask/src/bench/workloads``.

A script run on its own, ``python SCRIPT.py OUT [KEY=VALUE...]``, draws its
figure and saves it to OUT in the format of OUT's extension, with the
``savefig`` keywords given (``bbox_inches=tight``); ``python SCRIPT.py
--layout`` lays the figure out without drawing it. The benchmark harness
(``spec/reference/bench/bench.py``) imports the scripts and calls their
``draw()`` itself.

This module imports numpy only: the scripts' one import of
``matplotlib.pyplot`` is the line that changes when they run under pltrs.
"""

import math
import sys

import numpy as np


class FastRng:
    """``spec_support.SpecRng``'s stream, drawn with numpy in bulk."""

    _GOLDEN = np.uint64(0x9E3779B97F4A7C15)

    def __init__(self, seed):
        self._seed = np.uint64(seed)
        self._drawn = 0

    def _next_f64(self, n):
        k = np.arange(self._drawn + 1, self._drawn + n + 1, dtype=np.uint64)
        self._drawn += n
        z = self._seed + k * self._GOLDEN
        z = (z ^ (z >> np.uint64(30))) * np.uint64(0xBF58476D1CE4E5B9)
        z = (z ^ (z >> np.uint64(27))) * np.uint64(0x94D049BB133111EB)
        z ^= z >> np.uint64(31)
        return (z >> np.uint64(11)).astype(np.float64) * (1.0 / (1 << 53))

    def uniform(self, n, low, high):
        """``n`` values uniform in ``[low, high)``."""
        return low + (high - low) * self._next_f64(n)

    def normal(self, n, loc, scale):
        """``n`` normal values: Box–Muller, two uniforms per sample, as
        ``SpecRng.normal``."""
        u = self._next_f64(2 * n)
        u1, u2 = 1.0 - u[0::2], u[1::2]
        return loc + scale * np.sqrt(-2.0 * np.log(u1)) * np.cos(2.0 * math.pi * u2)


def random_walk(n, seed):
    """``n`` points of a Gaussian random walk against ``0..n``."""
    return np.arange(n, dtype=float), np.cumsum(FastRng(seed).normal(n, 0.0, 1.0))


def smooth_field(n):
    """``sin(x) cos(y) + x / 10`` on an ``n × n`` grid over ``[-10, 10]²``."""
    t = np.linspace(-10.0, 10.0, n)
    return np.sin(t)[None, :] * np.cos(t)[:, None] + t[None, :] / 10.0


def correlated_normals(seed):
    """10⁶ normal x values and y values correlated with them."""
    rng = FastRng(seed)
    x = rng.normal(1_000_000, 0.0, 1.0)
    return x, 0.5 * x + rng.normal(1_000_000, 0.0, 1.0)


def normal_groups(groups, seed):
    """``groups`` samples of 1000 normal values, with growing means and
    spreads."""
    rng = FastRng(seed)
    return [rng.normal(1000, g / 10.0, 1.0 + g / 100.0) for g in range(groups)]


def swirl(n):
    """The axis values and the two components of a swirling vector field
    on an ``n × n`` grid over ``[-3, 3]²``."""
    t = np.linspace(-3.0, 3.0, n)
    x, y = np.meshgrid(t, t)
    return t, -1.0 - x * x + y, 1.0 + x - y * y


def waves(count, points):
    """``count`` phase-shifted sine waves of ``points`` points over
    ``[0, 1]``, each raised above the last: the x values and the waves."""
    x = np.linspace(0.0, 1.0, points)
    return x, [np.sin(2.0 * math.pi * (x + k / count)) + k / 50.0 for k in range(count)]


def finish(fig, target, options):
    """Save ``fig`` to ``target`` (a path or a file object) with the
    ``savefig`` keywords ``options``, or lay it out without drawing it
    when ``target`` is None."""
    if target is None:
        fig.draw_without_rendering()
    else:
        fig.savefig(target, **options)


def save(fig, args=None):
    """What a script does last: save ``fig`` as its command line asks
    (``OUT [KEY=VALUE...]``, or ``--layout``)."""
    args = sys.argv[1:] if args is None else args
    if args == ["--layout"]:
        finish(fig, None, {})
        return
    if not args:
        sys.exit(f"usage: {sys.argv[0]} OUT [KEY=VALUE...] | --layout")
    out, *options = args
    finish(fig, out, dict(option.split("=", 1) for option in options))
