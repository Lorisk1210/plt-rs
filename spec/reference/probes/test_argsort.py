"""numpy's argsort of float64 arrays, down to the order of equal values.

Area ``argsort``. matplotlib draws 3D faces and points in the order
``np.argsort`` gives their depths, and numpy's default sort is not stable:
which of two equal depths comes first, and so which face covers which, is
whatever numpy's algorithm leaves. numpy 2 on x86-64 with AVX-512 sorts
with x86-simd-sort: bitonic networks up to 256 values, vectorised
quicksort partitions above that, and libstdc++'s ``std::sort`` for arrays
with NaN or once the partitions stop making progress (e.g. when most
values are the minimum). The inputs below are full of ties and take each
of those paths.

Ops
---
``numpy.argsort``  {values} -> the indices ``np.argsort(values)`` returns
    for ``values`` as a float64 array (``NaN``/``Infinity`` allowed).
"""
import numpy as np

from spec_support import ProbeSet, rng

probes = ProbeSet("argsort")


def _argsort(values):
    return [int(i) for i in np.argsort(np.asarray(values, dtype=float))]


def _probe(values):
    probes.probe("numpy.argsort", _argsort, values=[float(v) for v in values])


def _levels(seed, n, levels):
    """``n`` values drawn from ``levels`` multiples of 1/4."""
    return rng(seed).integers(n, 0, levels) / 4


NETWORK_SIZES = [2, 3, 5, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 200,
                 255, 256]
PARTITION_SIZES = [257, 300, 511, 640, 1000, 1500, 2600]


def test_networks():
    for seed, n in enumerate(NETWORK_SIZES):
        for levels in (2, 3, 7):
            _probe(_levels(100 * seed + levels, n, levels))


def test_partitions():
    for seed, n in enumerate(PARTITION_SIZES):
        for levels in (2, 5, 40):
            _probe(_levels(1000 + 100 * seed + levels, n, levels))


def test_stalled_partitions():
    # Mostly the minimum (or the maximum): the median-of-eight pivot is an
    # extreme, the partitions make no progress and std::sort takes over.
    for n, every in ((600, 7), (2000, 11), (3000, 4)):
        _probe([float(i % 13) if i % every == 3 else 0.0 for i in range(n)])
        _probe([float(i % 13) if i % every == 3 else 20.0 for i in range(n)])


def test_nan_and_infinities():
    for seed, n in enumerate((7, 40, 300, 1200)):
        values = _levels(5000 + seed, n, 4)
        values[::5] = np.nan
        _probe(values)
        values = _levels(6000 + seed, n, 4)
        values[::3] = np.inf
        values[1::7] = -np.inf
        _probe(values)


def test_signed_zeros_and_descending_runs():
    _probe([0.0, -0.0] * 40 + [1.0, -1.0] * 30)
    _probe([float(v) for v in range(400, 0, -1)])
    _probe([float(v // 3) for v in range(900, 0, -1)])
