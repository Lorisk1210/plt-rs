"""matplotlib's own tests of ``matplotlib.gridspec``
(``matplotlib/tests/test_gridspec.py``): equality and ``repr`` of grids and
cell blocks, grid updates, ratio validation, ``SubplotParams`` and nested
grids.

Area ``mpl_gridspec``. Cell blocks are given as half-open ranges
``[[row_start, row_stop], [col_start, col_stop]]``; subplot parameters are
recorded as ``[left, bottom, right, top, wspace, hspace]``.

Ops
---
``SubplotSpec.__eq__``  {shape, first, second} -> whether the blocks
    ``first`` and ``second`` of one ``GridSpec(*shape)`` compare equal
    (``test_equal``).
``GridSpec.update``  {shape, changes} -> the subplot parameters
    ``GridSpec(*shape)`` sets itself after ``update(**changes)``, ``null``
    for those it leaves to the figure (``test_update``).
``GridSpec.__repr__``  {nrows, ncols, width_ratios?, height_ratios?} -> the
    ``repr`` of the grid given its ratios as tuples, or the error rejecting
    them
    (``test_repr``, ``test_width_ratios``, ``test_height_ratios``).
``SubplotSpec.__repr__``  {shape, block} -> the ``repr`` of a block of
    ``GridSpec(*shape)`` (``test_repr``).
``SubplotParams``  {left?, bottom?, right?, top?} -> the parameters of
    ``SubplotParams(...)``, unset ones from the rcParams, or the error
    (``test_SubplotParams``).
``SubplotParams.update``  {params, changes} -> the parameters of
    ``SubplotParams(*params)`` after ``update(**changes)``, or the error.
``SubplotParams.reset``  {params} -> the parameters of
    ``SubplotParams(*params)`` after ``reset()``.
``GridSpecFromSubplotSpec.get_topmost_subplotspec``  {shape, index, nested}
    -> the ``repr`` of the topmost block of a ``GridSpecFromSubplotSpec(*nested)``
    in axes ``index`` of ``fig.subplots(*shape)``, and whether it equals
    that axes' subplot spec (``test_subplotspec_args``).

Left out: the ``TypeError`` checks of ``test_subplotspec_args`` (an axes or
an array passed as ``subplot_spec``), which Rust's types rule out.
"""

import matplotlib.gridspec as gridspec
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_gridspec")

PARAMS = ["left", "bottom", "right", "top", "wspace", "hspace"]


def _block(gs, block):
    (r0, r1), (c0, c1) = block
    return gs[r0:r1, c0:c1]


def _params(obj):
    return [getattr(obj, name) for name in PARAMS]


def _equal(shape, first, second):
    gs = gridspec.GridSpec(*shape)
    return _block(gs, first) == _block(gs, second)


def _update(shape, changes):
    gs = gridspec.GridSpec(*shape)
    gs.update(**changes)
    return _params(gs)


def _grid_repr(nrows, ncols, width_ratios=None, height_ratios=None):
    # The upstream test gives the ratios as tuples, which JSON has not.
    as_tuple = lambda ratios: None if ratios is None else tuple(ratios)  # noqa: E731
    return repr(gridspec.GridSpec(nrows, ncols, width_ratios=as_tuple(width_ratios),
                                  height_ratios=as_tuple(height_ratios)))


def _block_repr(shape, block):
    return repr(_block(gridspec.GridSpec(*shape), block))


def _subplot_params(**params):
    return _params(gridspec.SubplotParams(**params))


def _subplot_params_update(params, changes):
    s = gridspec.SubplotParams(*params)
    s.update(**changes)
    return _params(s)


def _subplot_params_reset(params):
    s = gridspec.SubplotParams(*params)
    s.reset()
    return _params(s)


def _topmost(shape, index, nested):
    fig = Figure()
    axs = fig.subplots(*shape).ravel()
    spec = axs[index].get_subplotspec()
    gs = gridspec.GridSpecFromSubplotSpec(*nested, subplot_spec=spec)
    topmost = gs.get_topmost_subplotspec()
    return [repr(topmost), topmost == spec]


def test_equal():
    for first, second in [([[0, 1], [0, 1]], [[0, 1], [0, 1]]),
                          ([[0, 2], [0, 1]], [[0, 2], [0, 1]]),
                          ([[0, 1], [0, 1]], [[1, 2], [0, 1]]),
                          ([[0, 1], [0, 1]], [[0, 2], [0, 1]])]:
        probes.probe("SubplotSpec.__eq__", _equal, shape=[2, 1], first=first, second=second)


def test_update():
    for changes in [{"left": 0.1}, {"right": 0.8, "hspace": 0.4},
                    {"bottom": 0.2, "top": 0.7, "wspace": 0.0}]:
        probes.probe("GridSpec.update", _update, shape=[2, 1], changes=changes)


def test_repr():
    for args in [{"nrows": 3, "ncols": 3},
                 {"nrows": 2, "ncols": 2, "height_ratios": [3, 1], "width_ratios": [1, 3]},
                 {"nrows": 2, "ncols": 2, "height_ratios": [1, 1], "width_ratios": [2, 2]},
                 {"nrows": 1, "ncols": 2, "width_ratios": [0.5, 1.5]},
                 # test_width_ratios and test_height_ratios
                 {"nrows": 1, "ncols": 1, "width_ratios": [2, 1, 3]},
                 {"nrows": 1, "ncols": 1, "height_ratios": [2, 1, 3]}]:
        probes.probe("GridSpec.__repr__", _grid_repr, **args)
    for shape, block in [([3, 3], [[2, 3], [1, 3]]), ([2, 4], [[0, 2], [3, 4]]),
                         ([1, 1], [[0, 1], [0, 1]])]:
        probes.probe("SubplotSpec.__repr__", _block_repr, shape=shape, block=block)


def test_SubplotParams():
    for params in [{"left": 0.1, "bottom": 0.1, "right": 0.9, "top": 0.9}, {},
                   {"left": 0.3}, {"left": 0.1, "bottom": 0.1, "right": 0.09, "top": 0.9},
                   {"bottom": 0.95}]:
        probes.probe("SubplotParams", _subplot_params, **params)
    start = [0.1, 0.1, 0.9, 0.9]
    for changes in [{"left": 0.91}, {"bottom": 0.91}, {"left": 0.2, "top": 0.8},
                    {"wspace": 0.5, "hspace": 0.1}, {"right": 0.1}]:
        probes.probe("SubplotParams.update", _subplot_params_update, params=start,
                     changes=changes)
    probes.probe("SubplotParams.reset", _subplot_params_reset, params=start)


def test_subplotspec_args():
    for shape, index, nested in [([1, 2], 0, [2, 1]), ([1, 2], 1, [1, 3]),
                                 ([2, 2], 3, [2, 2])]:
        probes.probe("GridSpecFromSubplotSpec.get_topmost_subplotspec", _topmost,
                     shape=shape, index=index, nested=nested)
