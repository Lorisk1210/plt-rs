"""
What matplotlib's layout engines compute with their parameters before
they lay a figure out, for the errors it raises.

matplotlib's engines keep their parameters as given (`LayoutEngine.set`
stores anything but None) and compute with them only when they lay a
figure out, failing there on what they cannot compute with: a pad given
as a string, an empty list, a rectangle too short. pltrs (Rust) lays
figures out with numbers. An engine holding what pltrs (Rust) cannot
hold (`holds_tight`, `holds_constrained`) is laid out from Python before
every layout of a draw (`.LayoutEngine._executes_in_python`), whose
``execute`` computes with the parameters as matplotlib's does
(`check_tight`, `check_constrained`), raising its errors at the same
steps, before pltrs (Rust) lays the figure out with their values. A
`ValueError` there leaves the figure as it was laid out, as matplotlib's
draw passes over one of its layout engine.
"""

from numbers import Real

from . import _gridspec_kept, rcParams


def holds_tight(params):
    """Whether pltrs (Rust) holds the parameters of a tight layout engine:
    numbers (the pads between Axes None for *pad*) and a rectangle of four
    numbers."""
    return (_number(params['pad'])
            and all(params[pad] is None or _number(params[pad])
                    for pad in ('h_pad', 'w_pad'))
            and _rectangle(params['rect'], exactly=True))


def holds_constrained(params):
    """Whether pltrs (Rust) holds the parameters of a constrained layout
    engine: numbers, and a rectangle of four numbers or more (matplotlib
    reads the first four)."""
    return (all(_number(params[name])
                for name in ('h_pad', 'w_pad', 'hspace', 'wspace'))
            and _rectangle(params['rect'], exactly=False))


def tight_in_python(fig, params, grid_spec=None):
    """Whether matplotlib's tight layout of *fig* (of the Axes of
    *grid_spec* alone, if given) computes with what pltrs (Rust) cannot
    hold or computes otherwise: its parameters (`holds_tight`), or the
    grids of its Axes (`._gridspec_kept.in_python`)."""
    if not holds_tight(params):
        return True
    from ._tight_layout import get_subplotspec_list
    return any(spec is not None and _gridspec_kept.in_python(spec.get_gridspec(), fig)
               for spec in get_subplotspec_list(fig.axes, grid_spec=grid_spec))


def check_tight(fig, params, grid_spec=None):
    """
    Compute with the parameters of a tight layout engine laying *fig* out
    (the Axes of *grid_spec* alone, if given) as matplotlib's
    ``get_tight_layout_figure`` does (the boxes of the subplots, the pads,
    the decorations of the Axes measured, then the rectangle the subplots
    fit in), raising its errors; none for a figure without Axes in a grid,
    which it leaves as it is. Return whether it lays out any Axes.
    """
    from ._tight_layout import get_subplotspec_list
    specs = get_subplotspec_list(fig.axes, grid_spec=grid_spec)
    if all(spec is None for spec in specs):
        return False
    for spec in dict.fromkeys(spec for spec in specs if spec is not None):
        if _gridspec_kept.in_python(spec.get_gridspec(), fig):
            spec.get_position(fig)
    from .font_manager import FontProperties
    font_size_inch = FontProperties(
        size=rcParams["font.size"]).get_size_in_points() / 72
    params['pad'] * font_size_inch
    for pad in ('h_pad', 'w_pad'):
        if params[pad] is not None:
            params[pad] * font_size_inch
    rect = params['rect']
    if rect is not None:
        renderer = fig._get_renderer()
        for ax, spec in zip(fig.axes, specs):
            if spec is not None and ax.get_visible():
                ax.get_tightbbox(renderer, for_layout_only=True)
        # Fitted into the rectangle with the subplot parameters found
        # (fractions of the figure).
        left, bottom, right, top = rect
        for side, fitted in ((left, 0.0), (bottom, 0.0)):
            if side is not None:
                side + fitted
        for side, fitted in ((right, 1.0), (top, 1.0)):
            if side is not None:
                side - (1 - fitted)
    return True


def check_constrained(fig, params):
    """
    Compute with the parameters of a constrained layout engine laying
    *fig* out as matplotlib's ``ConstrainedLayoutEngine.execute`` and
    ``do_constrained_layout`` do (the pads in fractions of the figure, the
    rectangle of the figure's layout grid, the margins of each Axes in a
    grid), raising its errors.
    """
    width, height = fig.get_size_inches()
    w_pad = params['w_pad'] / width
    h_pad = params['h_pad'] / height
    rect = params['rect']
    # The constraints of the figure's layout grid (`parent_constraints`).
    rect[0], rect[0] + rect[2], rect[1] + rect[3], rect[1]
    for ax in fig.axes:
        spec = ax.get_subplotspec()
        if spec is None or not ax.get_in_layout():
            continue
        # The margins from the padding (`get_margin_from_padding`).
        grid = spec.get_gridspec()
        wspace, hspace = _spaces(grid)
        wspace = (params['wspace'] if wspace is None else wspace) / 2
        hspace = (params['hspace'] if hspace is None else hspace) / 2
        nrows, ncols = grid.get_geometry()
        bool(wspace / ncols > w_pad)
        bool(hspace / nrows > h_pad)


def _spaces(grid):
    """The spaces between the cells *grid* sets itself, None for those it
    leaves to the layout."""
    if hasattr(grid, 'hspace'):
        return grid.wspace, grid.hspace
    return grid._wspace, grid._hspace


def _number(value):
    """Whether *value* is a number pltrs (Rust) takes."""
    return isinstance(value, Real)


def _rectangle(rect, exactly):
    """Whether *rect* is a sequence of four numbers (*exactly* four, else
    four or more)."""
    try:
        count = len(rect)
        corners = [rect[i] for i in range(min(count, 4))]
    except (TypeError, KeyError, IndexError):
        return False
    return (count == 4 if exactly else count >= 4) and all(map(_number, corners))
