"""
`matplotlib.pyplot` is a state-based interface to matplotlib: implicit
"current" figures and axes that functions act on, as in MATLAB.

The figures pyplot manages are kept in `._pylab_helpers.Gcf`; each
function here acts on `gcf` or `gca`, as matplotlib's does.
"""

from . import _pylab_helpers, get_backend, rcParams, style  # noqa: F401  (matplotlib.style, as pyplot imports it)
from . import image  # noqa: F401  (matplotlib.image, as pyplot imports it)
from .backend_bases import FigureManagerBase
from .figure import Figure

__all__ = [
    "bar", "close", "figure", "gca", "gcf", "get_backend", "get_fignums",
    "get_figlabels", "legend", "plot", "rcParams", "savefig", "scatter",
    "show", "subplots", "switch_backend", "title", "xlabel", "ylabel",
]


def switch_backend(newbackend):
    """
    Set the pyplot backend.

    pltrs (Python) draws through pltrs (Rust) whatever the backend; closing the figures and
    recording the name is what switching does.
    """
    close("all")
    rcParams["backend"] = newbackend.lower()


def get_fignums():
    """Return a list of existing figure numbers."""
    return sorted(_pylab_helpers.Gcf.figs)


def get_figlabels():
    """Return a list of existing figure labels."""
    managers = _pylab_helpers.Gcf.get_all_fig_managers()
    managers.sort(key=lambda m: m.num)
    return [m.canvas.figure.get_label() for m in managers]


def figure(num=None, figsize=None, dpi=None, *, facecolor=None,
           edgecolor=None, frameon=True, FigureClass=Figure, clear=False,
           **kwargs):
    """
    Create a new figure, or activate an existing figure.

    Parameters
    ----------
    num : int or str or `.Figure`, optional
        A unique identifier for the figure: its number, its label, or the
        figure itself.
    """
    if isinstance(num, Figure):
        if num.canvas.manager is None:
            raise ValueError("The passed figure is not managed by pyplot")
        _pylab_helpers.Gcf._set_new_active_manager(num.canvas.manager)
        return num

    allnums = get_fignums()
    next_num = max(allnums) + 1 if allnums else 1
    fig_label = ''
    if num is None:
        num = next_num
    elif isinstance(num, str):
        fig_label = num
        all_labels = get_figlabels()
        if fig_label not in all_labels:
            num = next_num
        else:
            num = allnums[all_labels.index(fig_label)]
    else:
        num = int(num)

    manager = _pylab_helpers.Gcf.get_fig_manager(num)
    if manager is None:
        fig = FigureClass(figsize=figsize, dpi=dpi, facecolor=facecolor,
                          edgecolor=edgecolor, frameon=frameon, **kwargs)
        manager = FigureManagerBase(fig.canvas, num)
        if fig_label:
            fig.set_label(fig_label)
        _pylab_helpers.Gcf._set_new_active_manager(manager)
    return manager.canvas.figure


def gcf():
    """Get the current figure, creating one if there is none."""
    manager = _pylab_helpers.Gcf.get_active()
    if manager is not None:
        return manager.canvas.figure
    return figure()


def gca():
    """Get the current Axes, creating one if there is none."""
    return gcf().gca()


def close(fig=None):
    """
    Close a figure window, and unregister it from pyplot.

    Parameters
    ----------
    fig : None or int or str or `.Figure`
        The figure to close: the current figure (None), a figure number or
        label, a `.Figure`, or 'all'.
    """
    if fig is None:
        manager = _pylab_helpers.Gcf.get_active()
        if manager is not None:
            _pylab_helpers.Gcf.destroy(manager)
    elif fig == 'all':
        _pylab_helpers.Gcf.destroy_all()
    elif isinstance(fig, int):
        _pylab_helpers.Gcf.destroy(fig)
    elif isinstance(fig, str):
        all_labels = get_figlabels()
        if fig in all_labels:
            _pylab_helpers.Gcf.destroy(get_fignums()[all_labels.index(fig)])
    elif isinstance(fig, Figure):
        _pylab_helpers.Gcf.destroy_fig(fig)
    else:
        raise TypeError("close() argument must be a Figure, an int, a string, "
                        "or None, not %s" % type(fig))


def show(*args, **kwargs):
    """Display all open figures: without a GUI backend, nothing to do."""


def subplots(nrows=1, ncols=1, *, sharex=False, sharey=False, squeeze=True,
             width_ratios=None, height_ratios=None, subplot_kw=None,
             gridspec_kw=None, **fig_kw):
    """Create a figure and a set of subplots."""
    fig = figure(**fig_kw)
    axs = fig.subplots(nrows=nrows, ncols=ncols, sharex=sharex, sharey=sharey,
                       squeeze=squeeze, subplot_kw=subplot_kw,
                       gridspec_kw=gridspec_kw, height_ratios=height_ratios,
                       width_ratios=width_ratios)
    return fig, axs


def savefig(*args, **kwargs):
    """Save the current figure (`.Figure.savefig`)."""
    return gcf().savefig(*args, **kwargs)


def plot(*args, scalex=True, scaley=True, data=None, **kwargs):
    """Plot y versus x as lines and/or markers (`.Axes.plot`)."""
    return gca().plot(*args, scalex=scalex, scaley=scaley, data=data, **kwargs)


def scatter(x, y, s=None, c=None, **kwargs):
    """A scatter plot of *y* vs. *x* (`.Axes.scatter`)."""
    return gca().scatter(x, y, s=s, c=c, **kwargs)


def bar(x, height, width=0.8, bottom=None, *, align='center', data=None,
        **kwargs):
    """Make a bar plot (`.Axes.bar`)."""
    return gca().bar(x, height, width=width, bottom=bottom, align=align,
                     data=data, **kwargs)


def axhline(y=0, xmin=0, xmax=1, **kwargs):
    """Add a horizontal line spanning the whole or fraction of the Axes
    (`.Axes.axhline`)."""
    return gca().axhline(y=y, xmin=xmin, xmax=xmax, **kwargs)


def axvline(x=0, ymin=0, ymax=1, **kwargs):
    """Add a vertical line spanning the whole or fraction of the Axes
    (`.Axes.axvline`)."""
    return gca().axvline(x=x, ymin=ymin, ymax=ymax, **kwargs)


def axhspan(ymin, ymax, xmin=0, xmax=1, **kwargs):
    """Add a horizontal span (rectangle) across the Axes (`.Axes.axhspan`)."""
    return gca().axhspan(ymin, ymax, xmin=xmin, xmax=xmax, **kwargs)


def axvspan(xmin, xmax, ymin=0, ymax=1, **kwargs):
    """Add a vertical span (rectangle) across the Axes (`.Axes.axvspan`)."""
    return gca().axvspan(xmin, xmax, ymin=ymin, ymax=ymax, **kwargs)


def title(label, fontdict=None, loc=None, pad=None, *, y=None, **kwargs):
    """Set a title for the Axes (`.Axes.set_title`)."""
    return gca().set_title(label, fontdict=fontdict, loc=loc, pad=pad, y=y,
                           **kwargs)


def xlabel(xlabel, fontdict=None, labelpad=None, *, loc=None, **kwargs):
    """Set the label for the x-axis (`.Axes.set_xlabel`)."""
    return gca().set_xlabel(xlabel, fontdict=fontdict, labelpad=labelpad,
                            loc=loc, **kwargs)


def ylabel(ylabel, fontdict=None, labelpad=None, *, loc=None, **kwargs):
    """Set the label for the y-axis (`.Axes.set_ylabel`)."""
    return gca().set_ylabel(ylabel, fontdict=fontdict, labelpad=labelpad,
                            loc=loc, **kwargs)


def legend(*args, **kwargs):
    """Place a legend on the Axes (`.Axes.legend`)."""
    return gca().legend(*args, **kwargs)
