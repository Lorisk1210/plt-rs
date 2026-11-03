"""
pyplot's tick and scale functions, which ``pyplot`` exports: ``plt.xticks``,
``plt.yticks``, ``plt.rgrids``, ``plt.thetagrids`` (from matplotlib 3.11.2,
under the Matplotlib License), the wrappers of the Axes methods
``grid``, ``tick_params``, ``ticklabel_format``, ``locator_params``,
``minorticks_on``, ``minorticks_off``, ``xscale`` and ``yscale``, and the
tick helpers pyplot re-exports from ``ticker``.

As in `._pyplot_plotting`, the wrappers of Axes methods are made from the
methods themselves, keeping their signatures and docstrings.
"""

from ._pyplot_plotting import _wrapper
from .ticker import (  # noqa: F401  (pyplot exports them)
    TickHelper, Formatter, FixedFormatter, NullFormatter, FuncFormatter,
    FormatStrFormatter, ScalarFormatter, LogFormatter, LogFormatterExponent,
    LogFormatterMathtext, Locator, IndexLocator, FixedLocator, NullLocator,
    LinearLocator, LogLocator, AutoLocator, MultipleLocator, MaxNLocator)

grid = _wrapper("grid")
locator_params = _wrapper("locator_params")
loglog = _wrapper("loglog")
semilogx = _wrapper("semilogx")
semilogy = _wrapper("semilogy")
minorticks_off = _wrapper("minorticks_off")
minorticks_on = _wrapper("minorticks_on")
tick_params = _wrapper("tick_params")
ticklabel_format = _wrapper("ticklabel_format")


def _gca():
    from . import pyplot
    return pyplot.gca()


def xscale(value, **kwargs):
    """
    Set the xaxis' scale (`.Axes.set_xscale` on the current Axes).

    Parameters
    ----------
    value : str or `.ScaleBase`
        The axis scale type to apply.
    **kwargs
        If *value* is a string, keywords are passed to the instantiation
        method of the respective class.
    """
    _gca().set_xscale(value, **kwargs)


def yscale(value, **kwargs):
    """
    Set the yaxis' scale (`.Axes.set_yscale` on the current Axes).

    Parameters
    ----------
    value : str or `.ScaleBase`
        The axis scale type to apply.
    **kwargs
        If *value* is a string, keywords are passed to the instantiation
        method of the respective class.
    """
    _gca().set_yscale(value, **kwargs)


def _ticks(name, ticks, labels, minor, kwargs):
    """`xticks` (*name* ``"x"``) or `yticks`."""
    ax = _gca()
    if ticks is None:
        locs = getattr(ax, f"get_{name}ticks")(minor=minor)
        if labels is not None:
            raise TypeError(f"{name}ticks(): Parameter 'labels' can't be set "
                            "without setting 'ticks'")
    else:
        locs = getattr(ax, f"set_{name}ticks")(ticks, minor=minor)
    if labels is None:
        labels_out = getattr(ax, f"get_{name}ticklabels")(minor=minor)
        for label in labels_out:
            label._internal_update(kwargs)
    else:
        labels_out = getattr(ax, f"set_{name}ticklabels")(
            labels, minor=minor, **kwargs)
    return locs, labels_out


def xticks(ticks=None, labels=None, *, minor=False, **kwargs):
    """
    Get or set the current tick locations and labels of the x-axis.

    Pass no arguments to return the current values without modifying them.

    Parameters
    ----------
    ticks : array-like, optional
        The list of xtick locations.  Passing an empty list removes all xticks.
    labels : array-like, optional
        The labels to place at the given *ticks* locations.  This argument can
        only be passed if *ticks* is passed as well.
    minor : bool, default: False
        If ``False``, get/set the major ticks/labels; if ``True``, the minor
        ticks/labels.
    **kwargs
        `.Text` properties can be used to control the appearance of the labels.

    Returns
    -------
    locs
        The list of xtick locations.
    labels
        The list of xlabel `.Text` objects.
    """
    return _ticks("x", ticks, labels, minor, kwargs)


def yticks(ticks=None, labels=None, *, minor=False, **kwargs):
    """
    Get or set the current tick locations and labels of the y-axis.

    Pass no arguments to return the current values without modifying them.

    Parameters
    ----------
    ticks : array-like, optional
        The list of ytick locations.  Passing an empty list removes all yticks.
    labels : array-like, optional
        The labels to place at the given *ticks* locations.  This argument can
        only be passed if *ticks* is passed as well.
    minor : bool, default: False
        If ``False``, get/set the major ticks/labels; if ``True``, the minor
        ticks/labels.
    **kwargs
        `.Text` properties can be used to control the appearance of the labels.

    Returns
    -------
    locs
        The list of ytick locations.
    labels
        The list of ylabel `.Text` objects.
    """
    return _ticks("y", ticks, labels, minor, kwargs)


def _polar_gca(name):
    """The current Axes, which must be polar for *name*."""
    from .projections.polar import PolarAxes
    ax = _gca()
    if not isinstance(ax, PolarAxes):
        raise RuntimeError(f'{name} only defined for polar Axes')
    return ax


def rgrids(radii=None, labels=None, angle=None, fmt=None, **kwargs):
    """
    Get or set the radial gridlines on the current polar plot.

    Call signatures::

     lines, labels = rgrids()
     lines, labels = rgrids(radii, labels=None, angle=22.5, fmt=None, **kwargs)

    When called with no arguments, `.rgrids` simply returns the tuple
    (*lines*, *labels*). When called with arguments, the labels will
    appear at the specified radial distances and angle.

    Returns
    -------
    lines : list of `.lines.Line2D`
        The radial gridlines.
    labels : list of `.text.Text`
        The tick labels.
    """
    ax = _polar_gca('rgrids')
    if all(p is None for p in [radii, labels, angle, fmt]) and not kwargs:
        lines = ax.yaxis.get_gridlines()
        labels = ax.yaxis.get_ticklabels()
    elif radii is None:
        raise TypeError("'radii' cannot be None when other parameters are "
                        "passed")
    else:
        lines, labels = ax.set_rgrids(
            radii, labels=labels, angle=angle, fmt=fmt, **kwargs)
    return lines, labels


def thetagrids(angles=None, labels=None, fmt=None, **kwargs):
    """
    Get or set the theta gridlines on the current polar plot.

    Call signatures::

     lines, labels = thetagrids()
     lines, labels = thetagrids(angles, labels=None, fmt=None, **kwargs)

    When called with no arguments, `.thetagrids` simply returns the tuple
    (*lines*, *labels*). When called with arguments, the labels will
    appear at the specified angles.

    Returns
    -------
    lines : list of `.lines.Line2D`
        The theta gridlines.
    labels : list of `.text.Text`
        The tick labels.
    """
    ax = _polar_gca('thetagrids')
    if all(param is None for param in [angles, labels, fmt]) and not kwargs:
        lines = ax.xaxis.get_ticklines()
        labels = ax.xaxis.get_ticklabels()
    elif angles is None:
        raise TypeError("'angles' cannot be None when other parameters are "
                        "passed")
    else:
        lines, labels = ax.set_thetagrids(angles, labels=labels, fmt=fmt,
                                          **kwargs)
    return lines, labels


__all__ = [
    "AutoLocator", "FixedFormatter", "FixedLocator", "FormatStrFormatter",
    "Formatter", "FuncFormatter", "IndexLocator", "LinearLocator", "Locator",
    "LogFormatter", "LogFormatterExponent", "LogFormatterMathtext",
    "LogLocator", "MaxNLocator", "MultipleLocator", "NullFormatter",
    "NullLocator", "ScalarFormatter", "TickHelper", "grid", "locator_params",
    "loglog", "semilogx", "semilogy",
    "minorticks_off", "minorticks_on", "rgrids", "thetagrids", "tick_params",
    "ticklabel_format", "xscale", "xticks", "yscale", "yticks",
]

# matplotlib defines these in pyplot itself (``get_plot_commands`` lists
# them).
for _function in (rgrids, thetagrids, xscale, xticks, yscale, yticks):
    _function.__module__ = __name__.rpartition(".")[0] + ".pyplot"
