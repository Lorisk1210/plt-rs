"""
pyplot's layout function, which ``pyplot`` exports: ``plt.tight_layout``,
on the current Figure.
"""

from ._pyplot_figure import _copy_docstring_and_deprecators
from .figure import Figure


def _current_figure():
    """The current Figure (pyplot's `gcf`)."""
    from . import pyplot
    return pyplot.gcf()


@_copy_docstring_and_deprecators(Figure.tight_layout)
def tight_layout(*, pad=1.08, h_pad=None, w_pad=None, rect=None):
    _current_figure().tight_layout(pad=pad, h_pad=h_pad, w_pad=w_pad,
                                   rect=rect)


tight_layout.__module__ = __name__.rpartition(".")[0] + ".pyplot"

__all__ = ["tight_layout"]
