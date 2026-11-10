"""
Constrained layout (matplotlib's ``_constrained_layout``): adjust subplot
layouts so that there are no overlapping Axes or Axes decorations.

pltrs (Rust) lays the figure out (``pltrs::layout``): `do_constrained_layout`
gives it the layout's parameters, after the arithmetic matplotlib's does
with them, which raises its errors. The signature and docstring are
matplotlib 3.11.2's, under the Matplotlib License.
"""

from . import _layoutgrid, _lazy_imports

_layout_checks = _lazy_imports.module(globals(), "._layout_checks")


def do_constrained_layout(fig, h_pad, w_pad,
                          hspace=None, wspace=None, rect=(0, 0, 1, 1),
                          compress=False):
    """
    Do the constrained_layout.  Called at draw time in
     ``figure.constrained_layout()``

    Parameters
    ----------
    fig : `~matplotlib.figure.Figure`
        `.Figure` instance to do the layout in.

    h_pad, w_pad : float
      Padding around the Axes elements in figure-normalized units.

    hspace, wspace : float
       Fraction of the figure to dedicate to space between the
       Axes.  These are evenly spread between the gaps between the Axes.
       A value of 0.2 for a three-column layout would have a space
       of 0.1 of the figure width between each column.
       If h/wspace < h/w_pad, then the pads are used instead.

    rect : tuple of 4 floats
        Rectangle in figure coordinates to perform constrained layout in
        [left, bottom, width, height], each from 0-1.

    compress : bool
        Whether to shift Axes so that white space in between them is
        removed. This is useful for simple grids of fixed-aspect Axes (e.g.
        a grid of images).

    Returns
    -------
    layoutgrid : private debugging structure
    """
    params = {'h_pad': h_pad, 'w_pad': w_pad, 'hspace': hspace,
              'wspace': wspace, 'rect': rect}
    _layout_checks.check_constrained_layout(fig, w_pad, h_pad, params)
    # pltrs (Rust) takes the pads in inches; a space left None is one every
    # grid sets itself (matplotlib's arithmetic fails on it otherwise).
    width, height = fig.get_size_inches()
    spaces = [0. if space is None else float(space) for space in (hspace, wspace)]
    return _layoutgrid.execute(fig, (
        "constrained", (float(h_pad) * height, float(w_pad) * width, *spaces),
        tuple(float(rect[i]) for i in range(4)), bool(compress)))
