"""
pyplot's text functions, which ``pyplot`` exports: ``plt.text`` and
``plt.annotate`` (on the current Axes) and ``plt.figtext`` (on the current
Figure).

As in `._pyplot_plotting`, the wrappers of Axes methods are made from the
methods themselves, keeping their signatures and docstrings.
"""

import functools

from ._pyplot_plotting import _wrapper
from .figure import Figure

text = _wrapper("text")
annotate = _wrapper("annotate")


@functools.wraps(Figure.text)
def figtext(x, y, s, fontdict=None, **kwargs):
    from . import pyplot
    return pyplot.gcf().text(x, y, s, fontdict=fontdict, **kwargs)


figtext.__module__ = __name__.rpartition(".")[0] + ".pyplot"

__all__ = ["annotate", "figtext", "text"]
