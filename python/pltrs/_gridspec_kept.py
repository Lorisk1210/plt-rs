"""
The values matplotlib's grids and subplot parameters keep unchecked.

matplotlib's `.GridSpec` keeps its margins and spacing as they are given,
a `.GridSpecFromSubplotSpec` its spacing, `~.GridSpecBase.set_width_ratios`
and `~.GridSpecBase.set_height_ratios` any sequence of the grid's length,
and a figure's `.SubplotParams` any spacing (`~.SubplotParams.update`
compares only the margins). It computes with them only when it places the
cells of a grid (`~.GridSpecBase.get_grid_positions`), where a spacing of
``'0.2'`` fails with ``unsupported operand type(s) for +: 'int' and
'str'``, or a subfigure on them (``SubFigure._redo_transform_rel_fig``).

pltrs (Rust) holds numbers. What it cannot hold (`holds`, `holds_ratios`)
stays with the Python object, pltrs (Rust) keeping the value it had (or
the default). While a grid, a grid it is nested in or the parameters of
the figure it is placed in keep such a value, or an infinite or NaN one,
whose arithmetic numpy warns about (`in_python`), the cells of the grid
are placed with matplotlib's arithmetic (`grid_positions`,
`subplot_params`), which raises its errors and issues numpy's warnings,
wherever matplotlib places them: when an Axes is added on the grid, when
the figure's subplot parameters or the grid's change (`check_positions`),
and when a subfigure is added on the grid (`subfigure_bounds`).
"""

import math
from numbers import Real

import numpy as np

from . import _lazy_imports, rcParams
from ._native import gridspec as _gridspec
from ._program_classes import takes_from_program

_figure = _lazy_imports.module(globals(), ".figure", "_figure")

#: The margin and spacing names of a grid and of subplot parameters.
KEYS = ("left", "bottom", "right", "top", "wspace", "hspace")


def holds(value):
    """Whether pltrs (Rust) holds *value* as a margin or a spacing: a
    number, or None (none given)."""
    return value is None or type(value) in _PLAIN or isinstance(value, (Real, np.bool_))


#: Types `holds` and `_plain` answer for without the abstract base class
#: check (a figure's subplots check its grid's ratios at every add).
_PLAIN = frozenset({float, int})


def holds_ratios(ratios):
    """Whether pltrs (Rust) holds *ratios*, the relative widths or heights
    of a grid's columns or rows: numbers."""
    return all(isinstance(ratio, (Real, np.bool_)) for ratio in ratios)


def held(values):
    """*values*, margins and spacings, with None (none given) for those
    pltrs (Rust) cannot hold."""
    return tuple(value if holds(value) else None for value in values)


def _plain(value):
    """Whether pltrs (Rust) computes with *value*, a margin, a spacing or a
    ratio, as matplotlib does: a finite number, or None."""
    if type(value) is float:
        return math.isfinite(value)
    return value is None or holds(value) and np.isfinite(value)


def native_ratios(ratios, count):
    """*ratios* as pltrs (Rust) takes them: floats, or *count* equal ones
    for ratios it cannot hold (which the Python grid keeps)."""
    if holds_ratios(ratios):
        return [float(ratio) for ratio in ratios]
    return [1.0] * count


def _plain_grid(gridspec):
    """Whether pltrs (Rust) computes with the ratios, margins and spacing
    of *gridspec* itself as matplotlib does (`_plain`)."""
    attributes = gridspec.__dict__
    values = [*attributes.get("_col_width_ratios", ()),
              *attributes.get("_row_height_ratios", ())]
    if "_subplot_spec" in attributes:
        values += [attributes.get("_wspace"), attributes.get("_hspace")]
    elif "_kept_params" in attributes:
        if attributes["_kept_params"]:
            return False
        values += _gridspec.overrides(gridspec._native)
    return _all_plain(values)


def _all_plain(values):
    """Whether every one of *values* is `_plain`: told at once for None
    and for floats and ints numpy holds as int64 with a finite sum (a NaN
    or an infinity makes the sum none), value by value otherwise. A figure
    checks its grid's ratios at every subplot it adds."""
    if set(map(type, values)) <= _PLAIN_OR_NONE:
        numbers = [value for value in values if value is not None]
        try:
            if not numbers or (math.isfinite(math.fsum(numbers))
                               and -_INT64 <= min(numbers) and max(numbers) < _INT64):
                return True
        except (OverflowError, ValueError):
            pass
    return all(map(_plain, values))


#: Where numpy's int64 ends: larger ints are objects to `np.isfinite`.
_INT64 = 2**63

#: The types of values `_all_plain` answers for at once.
_PLAIN_OR_NONE = frozenset({float, int, type(None)})


def in_python(gridspec, figure):
    """
    Whether the cells of *gridspec* in *figure* are placed with
    matplotlib's arithmetic: whether the grid, a grid it is nested in, or
    the subplot parameters of *figure* hold a value pltrs (Rust) does not
    compute with as matplotlib does (`_plain`), or *figure* is no figure
    (matplotlib fails reading its parameters).
    """
    while True:
        if not _plain_grid(gridspec):
            return True
        parent = gridspec.__dict__.get("_subplot_spec")
        if parent is None:
            break
        gridspec = parent.get_gridspec()
    if figure is None:
        return False
    if not isinstance(figure, _figure.FigureBase):
        return True
    params = figure.subplotpars
    return not _all_plain([getattr(params, key) for key in KEYS])


def subplot_params(gridspec, figure):
    """The `.SubplotParams` of the `.GridSpec` *gridspec* in *figure*, as
    matplotlib's `~.GridSpec.get_subplot_params` makes them: the grid's own
    values over those of *figure* (else of rcParams), checked as
    `.SubplotParams.update` checks them."""
    from .gridspec import SubplotParams
    if figure is None:
        params = SubplotParams(
            **{key: rcParams["figure.subplot." + key] for key in KEYS})
    else:
        # A copy of the figure's parameters, which its update leaves as
        # they are (matplotlib's ``copy.copy(figure.subplotpars)``).
        given = figure.subplotpars
        params = SubplotParams.__new__(SubplotParams)
        params.__dict__.update({key: getattr(given, key) for key in KEYS})
    params.update(**{key: getattr(gridspec, key) for key in KEYS})
    return params


def grid_positions(gridspec, fig):
    """The bottoms, tops, lefts and rights of the cells of *gridspec* in
    *fig*, as matplotlib's `~.GridSpecBase.get_grid_positions` computes
    them."""
    nrows, ncols = gridspec.get_geometry()
    subplot_params = gridspec.get_subplot_params(fig)
    left = subplot_params.left
    right = subplot_params.right
    bottom = subplot_params.bottom
    top = subplot_params.top
    wspace = subplot_params.wspace
    hspace = subplot_params.hspace
    tot_width = right - left
    tot_height = top - bottom

    # calculate accumulated heights of columns
    cell_h = tot_height / (nrows + hspace*(nrows-1))
    sep_h = hspace * cell_h
    norm = cell_h * nrows / sum(gridspec._row_height_ratios)
    cell_heights = [r * norm for r in gridspec._row_height_ratios]
    sep_heights = [0] + ([sep_h] * (nrows-1))
    cell_hs = np.cumsum(np.column_stack([sep_heights, cell_heights]).flat)

    # calculate accumulated widths of rows
    cell_w = tot_width / (ncols + wspace*(ncols-1))
    sep_w = wspace * cell_w
    norm = cell_w * ncols / sum(gridspec._col_width_ratios)
    cell_widths = [r * norm for r in gridspec._col_width_ratios]
    sep_widths = [0] + ([sep_w] * (ncols-1))
    cell_ws = np.cumsum(np.column_stack([sep_widths, cell_widths]).flat)

    fig_tops, fig_bottoms = (top - cell_hs).reshape((-1, 2)).T
    fig_lefts, fig_rights = (left + cell_ws).reshape((-1, 2)).T
    return fig_bottoms, fig_tops, fig_lefts, fig_rights


def check_positions(axes, figure=None):
    """Place each of *axes* on its subplot spec, in *figure* (else its own
    figure), as matplotlib places them when subplot parameters change
    (``ax._set_position(ax.get_subplotspec().get_position(fig))``), for
    its errors, where the spec's grid is placed in Python (`in_python`),
    and to where the program's code places it (`placed_by_program`);
    pltrs (Rust) places the others."""
    for ax in axes:
        spec = ax.get_subplotspec()
        if spec is None:
            continue
        fig = ax.get_figure(root=False) if figure is None else figure
        if placed_by_program(spec):
            ax._set_position(spec.get_position(fig))
        elif in_python(spec.get_gridspec(), fig):
            spec.get_position(fig)


def placed_by_program(subplotspec):
    """Whether the program's code places what is added on *subplotspec*:
    its class' `~.SubplotSpec.get_position`, or its grid's
    `~.GridSpecBase.get_grid_positions`, which matplotlib's `.Axes` asks
    for its position when it gets its spec and when subplot parameters
    change; pltrs (Rust) places the others."""
    return (takes_from_program(type(subplotspec), "get_position")
            or takes_from_program(type(subplotspec.get_gridspec()),
                                  "get_grid_positions"))


def subfigure_bounds(spec):
    """The bounds ``(x0, y0, width, height)`` of a subfigure on the cells of
    *spec* in its parent, from the ratios of the grid, as matplotlib's
    ``SubFigure._redo_transform_rel_fig`` computes them (with numpy's
    warnings)."""
    gs = spec.get_gridspec()
    wr = np.asarray(gs.get_width_ratios())
    hr = np.asarray(gs.get_height_ratios())
    dx = wr[spec.colspan].sum() / wr.sum()
    dy = hr[spec.rowspan].sum() / hr.sum()
    x0 = wr[:spec.colspan.start].sum() / wr.sum()
    y0 = 1 - hr[:spec.rowspan.stop].sum() / hr.sum()
    return x0, y0, dx, dy
