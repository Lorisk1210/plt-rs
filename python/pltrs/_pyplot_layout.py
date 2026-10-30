"""
pyplot's layout functions, which ``pyplot`` exports: ``plt.tight_layout``,
``plt.supxlabel`` and ``plt.supylabel``, on the current Figure.
"""


def _current_figure():
    """The current Figure (pyplot's `gcf`)."""
    from . import pyplot
    return pyplot.gcf()


def tight_layout(*, pad=1.08, h_pad=None, w_pad=None, rect=None):
    """
    Adjust the padding between and around subplots of the current figure
    (`.Figure.tight_layout`).
    """
    _current_figure().tight_layout(pad=pad, h_pad=h_pad, w_pad=w_pad,
                                   rect=rect)


def supxlabel(t, **kwargs):
    """Add a centered super xlabel to the current figure
    (`.Figure.supxlabel`)."""
    return _current_figure().supxlabel(t, **kwargs)


def supylabel(t, **kwargs):
    """Add a centered super ylabel to the current figure
    (`.Figure.supylabel`)."""
    return _current_figure().supylabel(t, **kwargs)


for _function in (tight_layout, supxlabel, supylabel):
    _function.__module__ = __name__.rpartition(".")[0] + ".pyplot"

__all__ = ["supxlabel", "supylabel", "tight_layout"]
