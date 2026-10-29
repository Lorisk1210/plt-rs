"""contourpy's quad contour generators, vertex for vertex.

Area ``contour_generator``. matplotlib contours quad grids with one of
contourpy's algorithms (``contour.algorithm``, or ``algorithm=``):
``mpl2005``, ``mpl2014`` (the default), ``serial`` or ``threaded``. They
agree on the geometry but not on where lines and rings start or in which
order they come. The fields below are noisy and quantised, so saddles,
values exactly at a level, holes, masked points (with and without corner
masking) and chunk edges all occur.

Ops
---
``QuadContourGenerator.lines``  {algorithm, x, y, z, mask, corner_mask,
    chunk_size, level} -> ``{vertices, codes}``: ``create_contour(level)``
    concatenated as matplotlib does (no lines: no codes).
``QuadContourGenerator.filled``  {..., lower, upper} -> the same for
    ``create_filled_contour(lower, upper)``.

``z`` may hold NaN, masked like ``mask`` (``null`` = none) as contourpy
masks invalid values. ``threaded`` is probed without chunks only: with
several, its threads append their chunks' contours in whatever order they
finish, which differs from run to run (pltrs keeps ``serial``'s order).
"""
import contourpy
import numpy as np
import pytest

from spec_support import ProbeSet, rng

probes = ProbeSet("contour_generator")

ALGORITHMS = ("mpl2005", "mpl2014", "serial", "threaded")


def _generator(algorithm, x, y, z, mask, corner_mask, chunk_size):
    z = np.asarray(z, dtype=float)
    if mask is not None:
        z = np.ma.array(z, mask=np.asarray(mask, dtype=bool))
    return contourpy.contour_generator(
        np.asarray(x, dtype=float), np.asarray(y, dtype=float), z, name=algorithm,
        corner_mask=corner_mask, line_type=contourpy.LineType.SeparateCode,
        fill_type=contourpy.FillType.OuterCode, chunk_size=chunk_size)


def _path(vertices_and_codes):
    vertices, codes = vertices_and_codes
    if not len(vertices):
        return {"vertices": [], "codes": None}
    return {"vertices": np.concatenate(vertices).tolist(),
            "codes": [int(c) for c in np.concatenate(codes)]}


def lines_op(level, **grid):
    return _path(_generator(**grid).create_contour(level))


def filled_op(lower, upper, **grid):
    return _path(_generator(**grid).create_filled_contour(lower, upper))


def _quantised(seed, ny, nx, step=0.25):
    """Normal noise rounded to multiples of ``step``: many values sit
    exactly on a level."""
    return np.round(rng(seed).normal(ny * nx).reshape(ny, nx) / step) * step


def _mask(seed, ny, nx, fraction):
    return (rng(seed).uniform(ny * nx).reshape(ny, nx) < fraction).tolist()


def _index_coords(ny, nx):
    x, y = np.meshgrid(np.arange(nx, dtype=float), np.arange(ny, dtype=float))
    return x.tolist(), y.tolist()


def _uneven_coords(seed, ny, nx):
    r = rng(seed)
    x, y = np.meshgrid(np.cumsum(r.uniform(nx, 0.5, 1.5)), np.cumsum(r.uniform(ny, 0.5, 1.5)))
    return x.tolist(), y.tolist()


def _curved_coords(seed, ny, nx):
    """A curvilinear grid: the index grid with every point jittered by less
    than a third of a cell."""
    x, y = np.meshgrid(np.arange(nx, dtype=float), np.arange(ny, dtype=float))
    r = rng(seed)
    jitter = r.uniform(2 * ny * nx, -0.3, 0.3).reshape(2, ny, nx)
    return (x + jitter[0]).tolist(), (y + jitter[1]).tolist()


def _rings(ny, nx):
    """Concentric ripples, quantised: bands with holes inside holes."""
    x, y = np.meshgrid(np.linspace(-3, 3, nx), np.linspace(-3, 3, ny))
    return np.round(np.cos(np.hypot(x, y) * 1.7) * 8) / 8


def _with_nan(z, seed, fraction):
    z = np.array(z, dtype=float)
    z[np.asarray(_mask(seed, *z.shape, fraction))] = np.nan
    return z


NOISE_LEVELS = [-1.0, -0.5, 0.0, 0.25, 1.0]
RING_LEVELS = [-0.75, -0.25, 0.25, 0.5, 0.875]


def _case(z, coords, levels, mask=None, chunks=(0,)):
    x, y = coords
    return {"x": x, "y": y, "z": np.asarray(z, dtype=float).tolist(), "mask": mask,
            "levels": levels, "chunks": chunks}


CASES = [
    ("tiny", _case(_quantised(1, 3, 4), _index_coords(3, 4), NOISE_LEVELS)),
    ("noise", _case(_quantised(2, 9, 11), _index_coords(9, 11), NOISE_LEVELS, chunks=(0, 3))),
    ("noise_fine", _case(_quantised(3, 14, 12, 0.5), _uneven_coords(4, 14, 12), NOISE_LEVELS)),
    ("noise_masked", _case(_quantised(5, 10, 12), _index_coords(10, 12), NOISE_LEVELS,
                           mask=_mask(6, 10, 12, 0.15), chunks=(0, 4))),
    ("noise_holes", _case(_quantised(7, 12, 9), _curved_coords(8, 12, 9), NOISE_LEVELS,
                          mask=_mask(9, 12, 9, 0.3))),
    ("noise_nan", _case(_with_nan(_quantised(10, 11, 10), 11, 0.1), _index_coords(11, 10),
                        NOISE_LEVELS, chunks=(0, 2))),
    ("rings", _case(_rings(17, 19), _index_coords(17, 19), RING_LEVELS, chunks=(0, 5))),
    ("rings_masked", _case(_rings(15, 15), _uneven_coords(12, 15, 15), RING_LEVELS,
                           mask=_mask(13, 15, 15, 0.08))),
    ("flat", _case(np.ones((4, 5)), _index_coords(4, 5), [0.0, 1.0, 2.0], chunks=(0, 2))),
]


def _grids(case):
    """The generator arguments of a case, for every algorithm, corner mask
    and chunk size."""
    for algorithm in ALGORITHMS:
        corner_masks = (True, False) if algorithm != "mpl2005" else (False,)
        if case["mask"] is None and not any(np.isnan(np.asarray(case["z"])).flat):
            corner_masks = corner_masks[:1]
        chunks = case["chunks"] if algorithm != "threaded" else (0,)
        for corner_mask in corner_masks:
            for chunk_size in chunks:
                yield {"algorithm": algorithm, "x": case["x"], "y": case["y"], "z": case["z"],
                       "mask": case["mask"], "corner_mask": corner_mask,
                       "chunk_size": chunk_size}


@pytest.mark.parametrize("case_id, case", CASES, ids=[c[0] for c in CASES])
def test_generators(case_id, case):
    levels = case["levels"]
    for grid in _grids(case):
        for level in levels:
            probes.probe("QuadContourGenerator.lines", lines_op, level=level, **grid)
        for lower, upper in zip(levels, levels[1:]):
            probes.probe("QuadContourGenerator.filled", filled_op, lower=lower, upper=upper,
                         **grid)


def test_corner_masks_need_a_capable_algorithm():
    x, y = _index_coords(2, 2)
    grid = {"algorithm": "mpl2005", "x": x, "y": y, "z": [[0.0, 1.0], [1.0, 2.0]],
            "mask": None, "corner_mask": True, "chunk_size": 0}
    probes.probe("QuadContourGenerator.lines", lines_op, level=0.5, **grid)
