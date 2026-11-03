"""
`matplotlib.pyplot` is a state-based interface to matplotlib: implicit
"current" figures and axes that functions act on, as in MATLAB.

The figures pyplot manages are kept in `._pylab_helpers.Gcf`; each
function here acts on `gcf` or `gca`, as matplotlib's does.
"""

# matplotlib's pyplot defers its annotations and exports both names.
from __future__ import annotations

from contextlib import ExitStack
import importlib
import inspect
import logging
from typing import TYPE_CHECKING  # noqa: F401

from . import _pylab_helpers, get_backend, rcParams, style  # noqa: F401  (matplotlib.style, as pyplot imports it)
from . import _api, cbook, interactive, is_interactive, set_loglevel  # noqa: F401
from . import (rc, rc_context, rcdefaults, rcParamsDefault,  # noqa: F401  (pyplot exports them)
               rc_file_defaults)
from . import image  # noqa: F401  (matplotlib.image, as pyplot imports it)
from . import rcParamsOrig, rcsetup
from .backend_bases import _Backend, FigureManagerBase
from .backends import backend_registry
from .figure import Figure, FigureBase, figaspect  # noqa: F401  (pyplot exports figaspect)
from .lines import Line2D  # noqa: F401  (pyplot exports it)
from .patches import Arrow, Circle, Polygon, Rectangle  # noqa: F401  (pyplot exports them)
from .gridspec import GridSpec, SubplotSpec

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

    if newbackend is rcsetup._auto_backend_sentinel:
        current_framework = cbook._get_running_interactive_framework()

        if (current_framework and
                (backend := backend_registry.backend_for_gui_framework(
                    current_framework))):
            candidates = [backend]
        else:
            candidates = []
        # pltrs' own windows come first where they open (``macosx`` stands
        # for them on macOS).
        candidates += [
            "macosx", "pltrs", "qtagg", "gtk4agg", "gtk3agg", "tkagg", "wxagg"]

        # Don't try to fallback on the cairo-based backends as they each have
        # an additional dependency (pycairo) over the agg-based backend, and
        # are of worse quality.
        for candidate in candidates:
            try:
                switch_backend(candidate)
            except ImportError:
                continue
            else:
                rcParamsOrig['backend'] = candidate
                return
        else:
            # Switching to Agg should always succeed; if it doesn't, let the
            # exception propagate out.
            switch_backend("agg")
            rcParamsOrig["backend"] = "agg"
            return

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
    allnums = get_fignums()
    next_num = max(allnums) + 1 if allnums else 1

    if isinstance(num, FigureBase):
        has_figure_property_parameters = (
            any(param is not None for param in [figsize, dpi, facecolor, edgecolor])
            or not frameon or kwargs
        )

        root_fig = num.get_figure(root=True)
        if root_fig.canvas.manager is None:
            if has_figure_property_parameters:
                raise ValueError(
                    "You cannot pass figure properties when calling figure() with "
                    "an existing Figure instance")
            backend = _get_backend_mod()
            manager = backend.new_figure_manager_given_figure(next_num, root_fig)
            _pylab_helpers.Gcf._set_new_active_manager(manager)
            return manager.canvas.figure
        elif has_figure_property_parameters and root_fig.canvas.manager.num in allnums:
            _api.warn_external(
                "Ignoring specified arguments in this call because figure "
                f"with num: {root_fig.canvas.manager.num} already exists")
        _pylab_helpers.Gcf.set_active(root_fig.canvas.manager)
        return root_fig

    fig_label = ''
    if num is None:
        num = next_num
    else:
        if (any(param is not None for param in [figsize, dpi, facecolor, edgecolor])
              or not frameon or kwargs) and num in allnums:
            _api.warn_external(
                "Ignoring specified arguments in this call "
                f"because figure with num: {num} already exists")
        if isinstance(num, str):
            fig_label = num
            all_labels = get_figlabels()
            if fig_label not in all_labels:
                if fig_label == 'all':
                    _api.warn_external("close('all') closes all existing figures.")
                num = next_num
            else:
                inum = all_labels.index(fig_label)
                num = allnums[inum]
        else:
            num = int(num)  # crude validation of num argument

    manager = _pylab_helpers.Gcf.get_fig_manager(num)
    if manager is None:
        max_open_warning = rcParams['figure.max_open_warning']
        if len(allnums) == max_open_warning >= 1:
            _api.warn_external(
                f"More than {max_open_warning} figures have been opened. "
                f"Figures created through the pyplot interface "
                f"(`matplotlib.pyplot.figure`) are retained until explicitly "
                f"closed and may consume too much memory. (To control this "
                f"warning, see the rcParam `figure.max_open_warning`). "
                f"Consider using `matplotlib.pyplot.close()`.",
                RuntimeWarning)
        manager = new_figure_manager(
            num, figsize=figsize, dpi=dpi, facecolor=facecolor,
            edgecolor=edgecolor, frameon=frameon, FigureClass=FigureClass,
            **kwargs)
        fig = manager.canvas.figure
        if fig_label:
            fig.set_label(fig_label)

        for hookspecs in rcParams["figure.hooks"]:
            module_name, dotted_name = hookspecs.split(":")
            obj = importlib.import_module(module_name)
            for part in dotted_name.split("."):
                obj = getattr(obj, part)
            obj(fig)

        _pylab_helpers.Gcf._set_new_active_manager(manager)
        draw_if_interactive()

    if clear:
        manager.canvas.figure.clear()

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
    elif hasattr(fig, 'int'):  # UUIDs get converted to ints by figure().
        _pylab_helpers.Gcf.destroy(fig.int)
    elif isinstance(fig, str):
        all_labels = get_figlabels()
        if fig in all_labels:
            _pylab_helpers.Gcf.destroy(get_fignums()[all_labels.index(fig)])
    elif isinstance(fig, Figure):
        _pylab_helpers.Gcf.destroy_fig(fig)
    else:
        _api.check_isinstance((Figure, int, str, None), fig=fig)


def _raise_if_figure_exists(num, func_name, clear=False):
    """
    Raise a ValueError if the figure *num* already exists.
    """
    if num is not None and not clear:
        if isinstance(num, FigureBase):
            raise ValueError(
                f"num {num!r} cannot be a FigureBase instance. "
                f"plt.{func_name}() is for creating new figures. "
                f"To add to an existing figure, use fig.{func_name}() "
                "instead.")

        if fignum_exists(num):
            raise ValueError(
                f"Figure {num!r} already exists. Use plt.figure({num!r}) "
                f"to get it or plt.close({num!r}) to close it. "
                f"Alternatively, pass 'clear=True' to {func_name}().")


def isinteractive():
    """
    Return whether plots are updated after every plotting command.
    """
    return is_interactive()


def ioff():
    """
    Disable interactive mode.

    For a temporary change, this can be used as a context manager::

        with plt.ioff():
            fig2 = plt.figure()
    """
    stack = ExitStack()
    stack.callback(ion if isinteractive() else ioff)
    interactive(False)
    return stack


def ion():
    """
    Enable interactive mode.

    For a temporary change, this can be used as a context manager::

        with plt.ion():
            fig2 = plt.figure()
    """
    stack = ExitStack()
    stack.callback(ion if isinteractive() else ioff)
    interactive(True)
    return stack


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
    _raise_if_figure_exists(fig_kw.get('num'), "subplots", fig_kw.get('clear'))
    fig = figure(**fig_kw)
    axs = fig.subplots(nrows=nrows, ncols=ncols, sharex=sharex, sharey=sharey,
                       squeeze=squeeze, subplot_kw=subplot_kw,
                       gridspec_kw=gridspec_kw, height_ratios=height_ratios,
                       width_ratios=width_ratios)
    return fig, axs


def savefig(fname, **kwargs):
    """Save the current figure (`.Figure.savefig`)."""
    fig = gcf()
    res = fig.savefig(fname, **kwargs)
    fig.canvas.draw_idle()  # Need this if 'transparent=True', to reset colors.
    return res


def plot(*args, scalex=True, scaley=True, data=None, **kwargs):
    """Plot y versus x as lines and/or markers (`.Axes.plot`)."""
    return gca().plot(*args, scalex=scalex, scaley=scaley, data=data, **kwargs)


def scatter(x, y, s=None, c=None, marker=None, cmap=None, norm=None,
            vmin=None, vmax=None, alpha=None, linewidths=None, *,
            edgecolors=None, colorizer=None, plotnonfinite=False, data=None,
            **kwargs):
    """
    A scatter plot of *y* vs. *x* (`.Axes.scatter`), which becomes the
    current image (`sci`).
    """
    ret = gca().scatter(
        x, y, s=s, c=c, marker=marker, cmap=cmap, norm=norm, vmin=vmin,
        vmax=vmax, alpha=alpha, linewidths=linewidths, edgecolors=edgecolors,
        colorizer=colorizer, plotnonfinite=plotnonfinite, data=data,
        **kwargs)
    sci(ret)
    return ret


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
           filterrad=4.0, resample=None, url=None, data=None, **kwargs):
    """Display data as an image (`.Axes.imshow`)."""
    return gca().imshow(
        X, cmap=cmap, norm=norm, aspect=aspect, interpolation=interpolation,
        alpha=alpha, vmin=vmin, vmax=vmax, colorizer=colorizer,
        origin=origin, extent=extent, interpolation_stage=interpolation_stage,
        filternorm=filternorm, filterrad=filterrad, resample=resample,
        url=url, data=data, **kwargs)


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


def axes(arg=None, **kwargs):
    """
    Add an Axes to the current figure and make it the current Axes.

    Call signatures::

        plt.axes()
        plt.axes(rect, projection=None, polar=False, **kwargs)
        plt.axes(ax)

    Parameters
    ----------
    arg : None or 4-tuple
        The exact behavior of this function depends on the type:

        - *None*: A new full window Axes is added using
          ``subplot(**kwargs)``.
        - 4-tuple of floats *rect* = ``(left, bottom, width, height)``.
          A new Axes is added with dimensions *rect* in normalized
          (0, 1) units using `~.Figure.add_axes` on the current figure.

    Returns
    -------
    `~.axes.Axes`, or a subclass of `~.axes.Axes`
    """
    fig = gcf()
    pos = kwargs.pop('position', None)
    if arg is None:
        if pos is None:
            return fig.add_subplot(**kwargs)
        else:
            return fig.add_axes(pos, **kwargs)
    else:
        return fig.add_axes(arg, **kwargs)


def subplot(*args, **kwargs):
    """
    Add an Axes to the current figure or retrieve an existing Axes.

    This is a wrapper of `.Figure.add_subplot` which provides additional
    behavior when working with the implicit API (see the notes section).

    Call signatures::

       subplot(nrows, ncols, index, **kwargs)
       subplot(pos, **kwargs)
       subplot(**kwargs)
       subplot(ax)

    Returns
    -------
    `~.axes.Axes`
    """
    # Here we will only normalize `polar=True` vs `projection='polar'` and let
    # downstream code deal with the rest.
    unset = object()
    projection = kwargs.get('projection', unset)
    polar = kwargs.pop('polar', unset)
    if polar is not unset and polar:
        # if we got mixed messages from the user, raise
        if projection is not unset and projection != 'polar':
            raise ValueError(
                f"polar={polar}, yet projection={projection!r}. "
                "Only one of these arguments should be supplied."
            )
        kwargs['projection'] = projection = 'polar'

    # if subplot called without arguments, create subplot(1, 1, 1)
    if len(args) == 0:
        args = (1, 1, 1)

    # This check was added because it is very easy to type subplot(1, 2, False)
    # when subplots(1, 2, False) was intended (sharex=False, that is). In most
    # cases, no error will ever occur, but mysterious behavior can result
    # because what was intended to be the sharex argument is instead treated as
    # a subplot index for subplot()
    if len(args) >= 3 and isinstance(args[2], bool):
        _api.warn_external("The subplot index argument to subplot() appears "
                           "to be a boolean. Did you intend to use "
                           "subplots()?")
    # Check for nrows and ncols, which are not valid subplot args:
    if 'nrows' in kwargs or 'ncols' in kwargs:
        raise TypeError("subplot() got an unexpected keyword argument 'ncols' "
                        "and/or 'nrows'.  Did you intend to call subplots()?")

    fig = gcf()

    # First, search for an existing subplot with a matching spec.
    key = SubplotSpec._from_subplot_args(fig, args)

    for ax in fig.axes:
        # If we found an Axes at the position, we can reuse it if the user passed no
        # kwargs or if the Axes class and kwargs are identical.
        if (ax.get_subplotspec() == key
            and (kwargs == {}
                 or (getattr(ax, "_projection_init", None)
                     == fig._process_projection_requirements(**kwargs)))):
            break
    else:
        # we have exhausted the known Axes and none match, make a new one!
        ax = fig.add_subplot(*args, **kwargs)

    fig.sca(ax)

    return ax


def subplot2grid(shape, loc, rowspan=1, colspan=1, fig=None, **kwargs):
    """
    Create a subplot at a specific location inside a regular grid.

    Parameters
    ----------
    shape : (int, int)
        Number of rows and of columns of the grid in which to place axis.
    loc : (int, int)
        Row number and column number of the axis location within the grid.
    rowspan : int, default: 1
        Number of rows for the axis to span downwards.
    colspan : int, default: 1
        Number of columns for the axis to span to the right.
    fig : `.Figure`, optional
        Figure to place the subplot in. Defaults to the current figure.
    **kwargs
        Additional keyword arguments are handed to `~.Figure.add_subplot`.

    Returns
    -------
    `~.axes.Axes`
    """
    if fig is None:
        fig = gcf()
    rows, cols = shape
    gs = GridSpec._check_gridspec_exists(fig, rows, cols)
    subplotspec = gs.new_subplotspec(loc, rowspan=rowspan, colspan=colspan)
    return fig.add_subplot(subplotspec, **kwargs)


def subplot_mosaic(mosaic, *, sharex=False, sharey=False,
                   width_ratios=None, height_ratios=None, empty_sentinel='.',
                   subplot_kw=None, gridspec_kw=None, per_subplot_kw=None,
                   **fig_kw):
    """
    Build a layout of Axes based on ASCII art or nested lists.

    See `.Figure.subplot_mosaic`; *fig_kw* go to `.pyplot.figure`.

    Returns
    -------
    fig : `.Figure`
       The new figure
    dict[label, Axes]
       A dictionary mapping the labels to the Axes objects.
    """
    _raise_if_figure_exists(fig_kw.get('num'), "subplot_mosaic", fig_kw.get('clear'))
    fig = figure(**fig_kw)
    ax_dict = fig.subplot_mosaic(
        mosaic,
        sharex=sharex, sharey=sharey,
        height_ratios=height_ratios, width_ratios=width_ratios,
        subplot_kw=subplot_kw, gridspec_kw=gridspec_kw,
        empty_sentinel=empty_sentinel,
        per_subplot_kw=per_subplot_kw,
    )
    return fig, ax_dict


def twinx(ax=None):
    """
    Make and return a second Axes that shares the *x*-axis.  The new Axes will
    overlay *ax* (or the current Axes if *ax* is *None*), and its ticks will be
    on the right.
    """
    if ax is None:
        ax = gca()
    return ax.twinx()


def twiny(ax=None):
    """
    Make and return a second Axes that shares the *y*-axis.  The new Axes will
    overlay *ax* (or the current Axes if *ax* is *None*), and its ticks will be
    on the top.
    """
    if ax is None:
        ax = gca()
    return ax.twiny()


def sca(ax):
    """
    Set the current Axes to *ax* and the current Figure to the parent of *ax*.
    """
    fig = ax.get_figure(root=False)
    figure(fig)
    fig.sca(ax)


def subplots_adjust(left=None, bottom=None, right=None, top=None,
                    wspace=None, hspace=None):
    """Adjust the subplot layout parameters (`.Figure.subplots_adjust`)."""
    gcf().subplots_adjust(left=left, bottom=bottom, right=right, top=top,
                          wspace=wspace, hspace=hspace)


def getp(obj, *args, **kwargs):
    """Return the value of an `.Artist`'s property (`.artist.getp`)."""
    from . import artist
    return artist.getp(obj, *args, **kwargs)


def setp(obj, *args, **kwargs):
    """Set one or more properties on an `.Artist` (`.artist.setp`)."""
    from . import artist
    return artist.setp(obj, *args, **kwargs)


def suptitle(t, **kwargs):
    """Add a centered super title to the current figure (`.Figure.suptitle`)."""
    return gcf().suptitle(t, **kwargs)


def figtext(x, y, s, fontdict=None, **kwargs):
    """Add text to the current figure (`.Figure.text`)."""
    return gcf().text(x, y, s, fontdict=fontdict, **kwargs)


def axis(arg=None, /, *, emit=True, **kwargs):
    """Get or set some axis properties of the current Axes (`.Axes.axis`)."""
    return gca().axis(arg, emit=emit, **kwargs)


# The wrappers of the plotting methods (plt.hist, plt.errorbar, ...).
from ._pyplot_plotting import *  # noqa: E402,F401,F403
from ._pyplot_plotting import __all__ as _plotting_all  # noqa: E402
__all__ += _plotting_all

# The text functions (plt.text, plt.annotate, plt.figtext).
from ._pyplot_text import *  # noqa: E402,F401,F403
from ._pyplot_text import __all__ as _text_all  # noqa: E402
__all__ += _text_all

# The layout function (plt.tight_layout).
from ._pyplot_layout import *  # noqa: E402,F401,F403
from ._pyplot_layout import __all__ as _layout_all  # noqa: E402
__all__ += _layout_all

# The tick and scale functions (plt.xticks, plt.grid, plt.xscale, ...).
from ._pyplot_ticks import *  # noqa: E402,F401,F403
from ._pyplot_ticks import __all__ as _ticks_all  # noqa: E402
__all__ += _ticks_all

# The functions on the current Axes (plt.xlim, plt.cla, plt.matshow, ...).
from ._pyplot_axes import *  # noqa: E402,F401,F403
from ._pyplot_axes import __all__ as _axes_all  # noqa: E402
__all__ += _axes_all

# The functions on the current Figure and pyplot's state (plt.clf,
# plt.figimage, plt.xkcd, the colormap functions, ...).
from ._pyplot_figure import *  # noqa: E402,F401,F403
from ._pyplot_figure import __all__ as _figure_all  # noqa: E402
from ._pyplot_figure import _copy_docstring_and_deprecators  # noqa: E402,F401
__all__ += _figure_all

# The configuration functions (plt.rc, plt.rc_context, plt.rcdefaults,
# plt.set_loglevel), pyplot's own as in matplotlib.
from ._pyplot_rc import *  # noqa: E402,F401,F403

# What matplotlib's pyplot imports and so exports.
from cycler import cycler  # noqa: E402,F401
from . import mlab, rcsetup, rcParamsOrig  # noqa: E402,F401
from .artist import Artist  # noqa: E402,F401
from .axes import Axes, Subplot  # noqa: E402,F401
from .backend_bases import FigureCanvasBase, MouseButton  # noqa: E402,F401
from .backends import BackendFilter  # noqa: E402,F401
from .colorizer import Colorizer, ColorizingArtist  # noqa: E402,F401
from .projections import PolarAxes  # noqa: E402,F401
from .scale import get_scale_names  # noqa: E402,F401
from .text import Annotation, Text  # noqa: E402,F401
from .widgets import Button, Slider, Widget  # noqa: E402,F401

# If rcParams['backend_fallback'] is true, and an interactive backend is
# requested, ignore rcParams['backend'] and force selection of a backend that
# is compatible with the current running interactive framework (pltrs' own
# windows count among the interactive backends).
if rcParams["backend_fallback"]:
    requested_backend = rcParams._get_backend_or_none()
    requested_backend = None if requested_backend is None else requested_backend.lower()
    available_backends = [
        *backend_registry.list_builtin(BackendFilter.INTERACTIVE), "pltrs"]
    if (
        requested_backend in (set(available_backends) - {'webagg', 'nbagg'})
        and cbook._get_running_interactive_framework()
    ):
        rcParams._set("backend", rcsetup._auto_backend_sentinel)
