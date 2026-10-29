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
    "pcolor", "pcolormesh",
)


def _wrapper(name):
    """The pyplot function calling ``gca().<name>``."""
    method = getattr(Axes, name)

    @functools.wraps(method)
    def wrapper(*args, **kwargs):
        from . import pyplot
        return getattr(pyplot.gca(), name)(*args, **kwargs)

    signature = inspect.signature(method)
    wrapper.__signature__ = signature.replace(
        parameters=list(signature.parameters.values())[1:])
    wrapper.__module__ = __name__.rpartition(".")[0] + ".pyplot"
    return wrapper


__all__ = list(_METHODS)
globals().update({name: _wrapper(name) for name in _METHODS})
