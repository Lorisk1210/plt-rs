"""
`matplotlib.pyplot` is a state-based interface to matplotlib: implicit
"current" figures and axes that functions act on, as in MATLAB.

The figures pyplot manages are kept in `._pylab_helpers.Gcf`; each
function here acts on `gcf` or `gca`, as matplotlib's does.
"""

import inspect
import logging

from . import _pylab_helpers, get_backend, rcParams, style  # noqa: F401  (matplotlib.style, as pyplot imports it)
from . import cbook, rcParamsDefault
from . import image  # noqa: F401  (matplotlib.image, as pyplot imports it)
from .backend_bases import _Backend, FigureManagerBase
from .backends import backend_registry
from .figure import Figure

_log = logging.getLogger(__name__)

__all__ = [
    "bar", "close", "draw", "figure", "fignum_exists", "gca", "gcf",
    "get_backend", "get_current_fig_manager", "get_fignums", "get_figlabels",
    "legend", "new_figure_manager", "plot", "rcParams", "savefig", "scatter",
    "show", "subplots", "switch_backend", "title", "xlabel", "ylabel",
]


#: The backend pyplot uses, set by `switch_backend` (a `._Backend`
#: subclass made from the backend module).
_backend_mod = None


def _get_backend_mod():
    """
    Ensure that a backend is selected and return it.

    This is currently private, but may be made public in the future.
    """
    if _backend_mod is None:
        switch_backend(rcParams._get("backend"))
    return _backend_mod


def switch_backend(newbackend):
    """
    Set the pyplot backend.

    Switching to an interactive backend is possible only if no event loop for
    another interactive backend has started.  Switching to and from
    non-interactive backends is always possible.

    Parameters
    ----------
    newbackend : str
        The case-insensitive name of the backend to use.
    """
    global _backend_mod
    # make sure the init is pulled up so we can assign to it later
    from . import backends

    module = backend_registry.load_backend_module(newbackend)
    canvas_class = module.FigureCanvas

    required_framework = canvas_class.required_interactive_framework
    if required_framework is not None:
        current_framework = cbook._get_running_interactive_framework()
        if (current_framework and required_framework
                and current_framework != required_framework):
            raise ImportError(
                "Cannot load backend {!r} which requires the {!r} interactive "
                "framework, as {!r} is currently running".format(
                    newbackend, required_framework, current_framework))

    # Load the new_figure_manager() and show() functions from the backend.

    # Classically, backends can directly export these functions.  This should
    # keep working for backcompat.
    new_figure_manager = getattr(module, "new_figure_manager", None)
    show = getattr(module, "show", None)

    # In that classical approach, backends are implemented as modules, but
    # "inherit" default method implementations from backend_bases._Backend.
    # This is achieved by creating a "class" that inherits from
    # backend_bases._Backend and whose body is filled with the module globals.
    class backend_mod(_Backend):
        locals().update(vars(module))

    # However, the newer approach for defining new_figure_manager and
    # show is to derive them from canvas methods.  In that case, also
    # update backend_mod accordingly; also, per-backend customization of
    # draw_if_interactive is disabled.
    if new_figure_manager is None:

        def new_figure_manager_given_figure(num, figure):
            return canvas_class.new_manager(figure, num)

        def new_figure_manager(num, *args, FigureClass=Figure, **kwargs):
            fig = FigureClass(*args, **kwargs)
            return new_figure_manager_given_figure(num, fig)

        def draw_if_interactive():
            if rcParams["interactive"]:
                manager = _pylab_helpers.Gcf.get_active()
                if manager:
                    manager.canvas.draw_idle()

        backend_mod.new_figure_manager_given_figure = (
            new_figure_manager_given_figure)
        backend_mod.new_figure_manager = new_figure_manager
        backend_mod.draw_if_interactive = draw_if_interactive

    # If the manager explicitly overrides pyplot_show, use it even if a global
    # show is already present, as the latter may be here for backcompat.
    manager_class = getattr(canvas_class, "manager_class", None)
    # We can't compare directly manager_class.pyplot_show and FMB.pyplot_show
    # because pyplot_show is a classmethod so the above constructs are bound
    # classmethods, and thus always different (being bound to different
    # classes).  We also have to use getattr_static instead of vars as
    # manager_class could have no __dict__.
    manager_pyplot_show = inspect.getattr_static(manager_class, "pyplot_show", None)
    base_pyplot_show = inspect.getattr_static(FigureManagerBase, "pyplot_show", None)
    if (show is None
            or (manager_pyplot_show is not None
                and manager_pyplot_show != base_pyplot_show)):
        if not manager_pyplot_show:
            raise ValueError(
                f"Backend {newbackend} defines neither FigureCanvas.manager_class nor "
                f"a toplevel show function")
        backend_mod.show = manager_class.pyplot_show

    _log.debug("Loaded backend %s version %s.",
               newbackend, backend_mod.backend_version)

    rcParams['backend'] = rcParamsDefault['backend'] = newbackend
    _backend_mod = backend_mod
    for func_name in ["new_figure_manager", "draw_if_interactive", "show"]:
        globals()[func_name].__signature__ = inspect.signature(
            getattr(backend_mod, func_name))

    # Need to keep a global reference to the backend for compatibility reasons.
    # See https://github.com/matplotlib/matplotlib/issues/6092
    backends.backend = newbackend


def new_figure_manager(*args, **kwargs):
    """Create a new figure manager instance."""
    return _get_backend_mod().new_figure_manager(*args, **kwargs)


# This function's signature is rewritten upon backend-load by switch_backend.
def draw_if_interactive(*args, **kwargs):
    """
    Redraw the current figure if in interactive mode.

    .. warning::

        End users will typically not have to call this function because the
        the interactive mode takes care of this.
    """
    return _get_backend_mod().draw_if_interactive(*args, **kwargs)


def fignum_exists(num):
    """
    Return whether the figure with the given id exists.

    Parameters
    ----------
    num : int or str
        A figure identifier.

    Returns
    -------
    bool
        Whether or not a figure with id *num* exists.
    """
    return (
        _pylab_helpers.Gcf.has_fignum(num)
        if isinstance(num, int)
        else num in get_figlabels()
    )


def get_current_fig_manager():
    """
    Return the figure manager of the current figure.

    If no figure exists, a new one is created, and its figure manager is
    returned.
    """
    return gcf().canvas.manager


def draw():
    """
    Redraw the current figure.

    This is used to update a figure that has been altered, but not
    automatically re-drawn.
    """
    gcf().canvas.draw_idle()


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
        manager = new_figure_manager(
            num, figsize=figsize, dpi=dpi, facecolor=facecolor,
            edgecolor=edgecolor, frameon=frameon, FigureClass=FigureClass,
            **kwargs)
        fig = manager.canvas.figure
        if fig_label:
            fig.set_label(fig_label)
        _pylab_helpers.Gcf._set_new_active_manager(manager)
        draw_if_interactive()
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
    """
    Display all open figures.

    Parameters
    ----------
    block : bool, optional
        Whether to wait for all figures to be closed before returning.

        If `True` block and run the GUI main loop until all figure windows
        are closed.

        If `False` ensure that all figure windows are displayed and return
        immediately.  In this case, you are responsible for ensuring
        that the event loop is running to have responsive figures.

        Defaults to True in non-interactive mode and to False in interactive
        mode (see `.pyplot.isinteractive`).
    """
    return _get_backend_mod().show(*args, **kwargs)


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


# Colormaps, images and colorbars.
from . import cm  # noqa: E402, F401  (matplotlib.cm, as pyplot imports it)
from .cm import _colormaps as colormaps  # noqa: E402
from .colors import Colormap, Normalize  # noqa: E402, F401
from .colors import _color_sequences as color_sequences  # noqa: E402


def gci():
    """
    Get the current colorable artist.

    Specifically, returns the current `.ScalarMappable` instance (`.Image`
    created by `imshow` or `figimage`, `.Collection` created by `pcolor` or
    `scatter`, etc.), or *None* if no such instance has been defined.
    """
    return gcf()._gci()


def get_cmap(name=None, lut=None):
    """
    Get a colormap instance, defaulting to rc values if *name* is None.

    Parameters
    ----------
    name : `~matplotlib.colors.Colormap` or str or None, default: None
        If a `.Colormap` instance, it will be returned. Otherwise, the name of
        a colormap known to Matplotlib, which will be resampled by *lut*. The
        default, None, means :rc:`image.cmap`.
    lut : int or None, default: None
        If *name* is not already a Colormap instance and *lut* is not None, the
        colormap will be resampled to have *lut* entries in the lookup table.
    """
    from . import _api
    if name is None:
        name = rcParams['image.cmap']
    if isinstance(name, Colormap):
        return name
    _api.check_in_list(sorted(colormaps), name=name)
    if lut is None:
        return colormaps[name]
    else:
        return colormaps[name].resampled(lut)


def set_cmap(cmap):
    """
    Set the default colormap, and applies it to the current image if any.

    Parameters
    ----------
    cmap : `~matplotlib.colors.Colormap` or str
        A colormap instance or the name of a registered colormap.
    """
    from . import rc
    cmap = get_cmap(cmap)
    rc('image', cmap=cmap.name)
    im = gci()
    if im is not None:
        im.set_cmap(cmap)


def imshow(X, cmap=None, norm=None, *, aspect=None, interpolation=None,
           alpha=None, vmin=None, vmax=None, colorizer=None, origin=None,
           extent=None, interpolation_stage=None, filternorm=True,
           filterrad=4.0, resample=None, url=None, **kwargs):
    """Display data as an image (`.Axes.imshow`)."""
    return gca().imshow(
        X, cmap=cmap, norm=norm, aspect=aspect, interpolation=interpolation,
        alpha=alpha, vmin=vmin, vmax=vmax, colorizer=colorizer,
        origin=origin, extent=extent, interpolation_stage=interpolation_stage,
        filternorm=filternorm, filterrad=filterrad, resample=resample,
        url=url, **kwargs)


def imread(fname, format=None):
    """Read an image from a file into an array (`.image.imread`)."""
    return image.imread(fname, format)


def imsave(fname, arr, **kwargs):
    """Colormap and save an array as an image file (`.image.imsave`)."""
    image.imsave(fname, arr, **kwargs)


def colorbar(mappable=None, cax=None, ax=None, **kwargs):
    """Add a colorbar to the current image (`.Figure.colorbar`)."""
    if mappable is None:
        mappable = gci()
        if mappable is None:
            raise RuntimeError('No mappable was found to use for colorbar '
                               'creation. First define a mappable such as '
                               'an image (with imshow) or a contour set ('
                               'with contourf).')
    return gcf().colorbar(mappable, cax=cax, ax=ax, **kwargs)


def clim(vmin=None, vmax=None):
    """
    Set the color limits of the current image.

    If either *vmin* or *vmax* is None, the image min/max respectively
    will be used for color scaling.
    """
    im = gci()
    if im is None:
        raise RuntimeError('You must first define an image, e.g., with imshow')

    im.set_clim(vmin, vmax)
