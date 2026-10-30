"""
pyplot's wrappers of the plotting methods of `~.axes.Axes`
(``plt.hist``, ``plt.errorbar``, ...), which ``pyplot`` exports.

matplotlib generates these wrappers (``boilerplate.py``); each calls the
method of the same name on the current Axes (`.pyplot.gca`). Here they
are made from the methods themselves, so that each keeps its method's
signature (without ``self``) and docstring.
"""

import functools
import inspect

from .axes import Axes

#: The plotting methods pyplot wraps, by name.
_METHODS = (
    "axline", "barh", "broken_barh", "errorbar", "eventplot", "fill",
    "fill_between", "fill_betweenx", "hist", "hlines", "stackplot", "stairs",
    "stem", "step", "vlines", "boxplot", "bxp", "violinplot", "violin", "pie",
    "contour", "contourf", "clabel", "tricontour", "tricontourf", "triplot",
    "pcolor", "pcolormesh", "quiver", "quiverkey", "barbs", "streamplot",
    "bar_label", "grouped_bar", "pie_label", "hexbin", "hist2d", "pcolorfast",
    "spy", "psd", "csd", "cohere", "magnitude_spectrum", "angle_spectrum",
    "phase_spectrum", "specgram", "acorr", "xcorr", "ecdf", "tripcolor",
    "table",
)


#: The colour-mapped artist of what a plotting method returns, by method
#: name: pyplot makes it the current image (matplotlib's ``sci`` calls in
#: ``boilerplate.py``).
_MAPPABLES = {
    "pcolor": lambda ret: ret,
    "pcolormesh": lambda ret: ret,
    "pcolorfast": lambda ret: ret,
    "hexbin": lambda ret: ret,
    "tripcolor": lambda ret: ret,
    "quiver": lambda ret: ret,
    "hist2d": lambda ret: ret[-1],
    "specgram": lambda ret: ret[-1],
    "streamplot": lambda ret: ret.lines,
    "contour": lambda ret: ret if ret._A is not None else None,
    "contourf": lambda ret: ret if ret._A is not None else None,
    "tricontour": lambda ret: ret if ret._A is not None else None,
    "tricontourf": lambda ret: ret if ret._A is not None else None,
}


def _wrapper(name):
    """The pyplot function calling ``gca().<name>``."""
    method = getattr(Axes, name)
    mappable = _MAPPABLES.get(name)

    @functools.wraps(method)
    def wrapper(*args, **kwargs):
        from . import pyplot
        ax = pyplot.gca()
        ret = getattr(ax, name)(*args, **kwargs)
        if mappable is not None and mappable(ret) is not None:
            ax._sci(mappable(ret))
        return ret

    signature = inspect.signature(method)
    wrapper.__signature__ = signature.replace(
        parameters=list(signature.parameters.values())[1:])
    wrapper.__module__ = __name__.rpartition(".")[0] + ".pyplot"
    return wrapper


def polar(*args, **kwargs):
    """
    Make a polar plot.

    call signature::

      polar(theta, r, [fmt], **kwargs)

    This is a convenience wrapper around `.pyplot.plot`. It ensures that the
    current Axes is polar (or creates one if needed) and then passes all parameters
    to ``.pyplot.plot``.
    """
    from . import _api, pyplot
    from .projections.polar import PolarAxes
    fig = pyplot.gcf()
    if fig.get_axes():
        ax = pyplot.gca()
        if not isinstance(ax, PolarAxes):
            _api.warn_deprecated(
                "3.10",
                message="There exists a non-polar current Axes. Therefore, the "
                        "resulting plot from 'polar()' is non-polar. You likely "
                        "should call 'polar()' before any other pyplot plotting "
                        "commands. "
                        "Support for this scenario is deprecated in %(since)s and "
                        "will raise an error in %(removal)s"
            )
    else:
        ax = fig.add_subplot(projection="polar")
    return ax.plot(*args, **kwargs)


__all__ = [*_METHODS, "polar"]
globals().update({name: _wrapper(name) for name in _METHODS})
