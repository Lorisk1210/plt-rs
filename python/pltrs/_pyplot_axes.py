"""
pyplot's functions acting on the current Axes beyond plotting, which
``pyplot`` exports: its limits (``xlim``, ``ylim``, ``autoscale``,
``margins``), its box, clearing and removing it, ``arrow``, ``matshow``
and the current image (``sci``).

As in `._pyplot_plotting`, the wrappers of Axes methods are made from the
methods themselves, keeping their signatures and docstrings; the others
are matplotlib 3.11.2's, under the Matplotlib License.
"""

import numpy as np

from ._pyplot_figure import _copy_docstring_and_deprecators
from ._pyplot_plotting import _wrapper
from .axes import Axes

arrow = _wrapper("arrow")
autoscale = _wrapper("autoscale")
margins = _wrapper("margins")


def _gca():
    from . import pyplot
    return pyplot.gca()


def xlim(*args, **kwargs):
    """
    Get or set the x limits of the current Axes.

    Call signatures::

        left, right = xlim()  # return the current xlim
        xlim((left, right))   # set the xlim to left, right
        xlim(left, right)     # set the xlim to left, right

    If you do not specify args, you can pass *left* or *right* as kwargs,
    i.e.::

        xlim(right=3)  # adjust the right leaving left unchanged
        xlim(left=1)  # adjust the left leaving right unchanged

    Setting limits turns autoscaling off for the x-axis.

    Returns
    -------
    left, right
        A tuple of the new x-axis limits.

    Notes
    -----
    Calling this function with no arguments (e.g. ``xlim()``) is the pyplot
    equivalent of calling `~.Axes.get_xlim` on the current Axes.
    Calling this function with arguments is the pyplot equivalent of calling
    `~.Axes.set_xlim` on the current Axes. All arguments are passed though.
    """
    ax = _gca()
    if not args and not kwargs:
        return ax.get_xlim()
    return ax.set_xlim(*args, **kwargs)


def ylim(*args, **kwargs):
    """
    Get or set the y-limits of the current Axes.

    Call signatures::

        bottom, top = ylim()  # return the current ylim
        ylim((bottom, top))   # set the ylim to bottom, top
        ylim(bottom, top)     # set the ylim to bottom, top

    If you do not specify args, you can alternatively pass *bottom* or
    *top* as kwargs, i.e.::

        ylim(top=3)  # adjust the top leaving bottom unchanged
        ylim(bottom=1)  # adjust the bottom leaving top unchanged

    Setting limits turns autoscaling off for the y-axis.

    Returns
    -------
    bottom, top
        A tuple of the new y-axis limits.

    Notes
    -----
    Calling this function with no arguments (e.g. ``ylim()``) is the pyplot
    equivalent of calling `~.Axes.get_ylim` on the current Axes.
    Calling this function with arguments is the pyplot equivalent of calling
    `~.Axes.set_ylim` on the current Axes. All arguments are passed though.
    """
    ax = _gca()
    if not args and not kwargs:
        return ax.get_ylim()
    return ax.set_ylim(*args, **kwargs)


def box(on=None):
    """
    Turn the Axes box on or off on the current Axes.

    Parameters
    ----------
    on : bool or None
        The new `~matplotlib.axes.Axes` box state. If ``None``, toggle
        the state.

    See Also
    --------
    :meth:`matplotlib.axes.Axes.set_frame_on`
    :meth:`matplotlib.axes.Axes.get_frame_on`
    """
    ax = _gca()
    if on is None:
        on = not ax.get_frame_on()
    ax.set_frame_on(on)


def cla():
    """Clear the current Axes."""
    return _gca().cla()


def delaxes(ax=None):
    """
    Remove an `~.axes.Axes` (defaulting to the current Axes) from its figure.
    """
    if ax is None:
        ax = _gca()
    ax.remove()


@_copy_docstring_and_deprecators(Axes._sci)
def sci(im):
    _gca()._sci(im)


def matshow(A, fignum=None, **kwargs):
    """
    Display a 2D array as a matrix in a new figure window.

    The origin is set at the upper left hand corner.
    The indexing is ``(row, column)`` so that the first index runs vertically
    and the second index runs horizontally in the figure:

    .. code-block:: none

        A[0, 0]   ⋯ A[0, M-1]
           ⋮             ⋮
        A[N-1, 0] ⋯ A[N-1, M-1]

    The aspect ratio of the figure window is that of the array,
    unless this would make an excessively short or narrow figure.

    Tick labels for the xaxis are placed on top.

    Parameters
    ----------
    A : 2D array-like
        The matrix to be displayed.

    fignum : None or int
        If *None*, create a new, appropriately sized figure window.

        If 0, use the current Axes (creating one if there is none, without ever
        adjusting the figure size).

        Otherwise, create a new Axes on the figure with the given number
        (creating it at the appropriate size if it does not exist, but not
        adjusting the figure size otherwise).  Note that this will be drawn on
        top of any preexisting Axes on the figure.

    Returns
    -------
    `~matplotlib.image.AxesImage`

    Other Parameters
    ----------------
    **kwargs : `~matplotlib.axes.Axes.imshow` arguments

    """
    from . import pyplot
    A = np.asanyarray(A)
    if fignum == 0:
        ax = _gca()
    else:
        if fignum is not None and pyplot.fignum_exists(fignum):
            # Do not try to set a figure size.
            figsize = None
        else:
            # Extract actual aspect ratio of array and make appropriately
            # sized figure.
            figsize = pyplot.figaspect(A)
        fig = pyplot.figure(fignum, figsize=figsize)
        ax = fig.add_axes((0.15, 0.09, 0.775, 0.775))
    im = ax.matshow(A, **kwargs)
    sci(im)
    return im


for _function in (xlim, ylim, box, cla, delaxes, sci, matshow):
    _function.__module__ = __name__.rpartition(".")[0] + ".pyplot"

__all__ = ["arrow", "autoscale", "box", "cla", "delaxes", "margins",
           "matshow", "sci", "xlim", "ylim"]
