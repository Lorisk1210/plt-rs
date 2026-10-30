"""
pyplot's functions acting on the current Figure and on pyplot's state,
which ``pyplot`` exports: clearing the figure, ``figimage``,
``figlegend``, its events (``connect``, ``ginput``, ...), ``findobj``,
``xkcd``, the colormap functions (``plt.viridis``, ...), and the
helpers of pyplot's wrappers (``_copy_docstring_and_deprecators``,
``get_plot_commands``).

These are matplotlib 3.11.2's, under the Matplotlib License.
"""

from enum import Enum
import functools
import inspect
import sys
import time

from . import _api, _docstring, _pylab_helpers, artist, rcParams
from .backend_bases import MouseButton
from .figure import Figure


def _gcf():
    from . import pyplot
    return pyplot.gcf()


def _copy_docstring_and_deprecators(method, func=None):
    """
    Give the pyplot wrapper *func* the docstring of *method*, and the
    parameter renamings and keyword-only changes ``_api`` decorated
    *method* with; usable as a decorator factory without *func*.
    """
    if func is None:
        return functools.partial(_copy_docstring_and_deprecators, method)
    decorators = [_docstring.copy(method)]
    # Check whether the definition of *method* includes @_api.rename_parameter
    # or @_api.make_keyword_only decorators; if so, propagate them to the
    # pyplot wrapper as well.
    while hasattr(method, "__wrapped__"):
        potential_decorator = _api.deprecation.DECORATORS.get(method)
        if potential_decorator:
            decorators.append(potential_decorator)
        method = method.__wrapped__
    for decorator in decorators[::-1]:
        func = decorator(func)
    return func


# The state controlled by {,un}install_repl_displayhook().
_ReplDisplayHook = Enum("_ReplDisplayHook", ["NONE", "PLAIN", "IPYTHON"])
_REPL_DISPLAYHOOK = _ReplDisplayHook.NONE

draw_all = _pylab_helpers.Gcf.draw_all


def _draw_all_if_interactive():
    from . import is_interactive
    if is_interactive():
        draw_all()


def install_repl_displayhook():
    """
    Connect to the display hook of the current shell.

    The display hook gets called when the read-evaluate-print-loop (REPL) of
    the shell has finished the execution of a command. We use this callback
    to be able to automatically update a figure in interactive mode.

    This works both with IPython and with vanilla python shells.
    """
    global _REPL_DISPLAYHOOK

    if _REPL_DISPLAYHOOK is _ReplDisplayHook.IPYTHON:
        return
    mod_ipython = sys.modules.get("IPython")
    ip = mod_ipython.get_ipython() if mod_ipython else None
    if not ip:
        _REPL_DISPLAYHOOK = _ReplDisplayHook.PLAIN
        return
    ip.events.register("post_execute", _draw_all_if_interactive)
    _REPL_DISPLAYHOOK = _ReplDisplayHook.IPYTHON
    from .backends import backend_registry
    from . import get_backend
    _, ipython_gui_name = backend_registry.resolve_backend(get_backend())
    # trigger IPython's eventloop integration, if available
    if ipython_gui_name:
        ip.enable_gui(ipython_gui_name)


def uninstall_repl_displayhook():
    """Disconnect from the display hook of the current shell."""
    global _REPL_DISPLAYHOOK
    if _REPL_DISPLAYHOOK is _ReplDisplayHook.IPYTHON:
        from IPython import get_ipython
        ip = get_ipython()
        ip.events.unregister("post_execute", _draw_all_if_interactive)
    _REPL_DISPLAYHOOK = _ReplDisplayHook.NONE


@_copy_docstring_and_deprecators(artist.Artist.findobj)
def findobj(o=None, match=None, include_self=True):
    if o is None:
        o = _gcf()
    return o.findobj(match, include_self=include_self)


@_copy_docstring_and_deprecators(artist.get)
def get(obj, *args, **kwargs):
    return artist.get(obj, *args, **kwargs)


def pause(interval):
    """
    Run the GUI event loop for *interval* seconds.

    If there is an active figure, it will be updated and displayed before the
    pause, and the GUI event loop (if any) will run during the pause.

    This can be used for crude animation.  For more complex animation use
    :mod:`matplotlib.animation`.

    If there is no active figure, sleep for *interval* seconds instead.
    """
    manager = _pylab_helpers.Gcf.get_active()
    if manager is not None:
        from . import pyplot
        canvas = manager.canvas
        if canvas.figure.stale:
            canvas.draw_idle()
        pyplot.show(block=False)
        canvas.start_event_loop(interval)
    else:
        time.sleep(interval)


def xkcd(scale=1, length=100, randomness=2):
    """
    Turn on `xkcd <https://xkcd.com/>`_ sketch-style drawing mode.

    This will only have an effect on things drawn after this function is
    called. It works by a number of rcParams, overriding those set before;
    used as a context manager, it restores them on exit::

        with plt.xkcd():
            # This figure will be in XKCD-style
            fig1 = plt.figure()

    Parameters
    ----------
    scale : float, optional
        The amplitude of the wiggle perpendicular to the source line.
    length : float, optional
        The length of the wiggle along the line.
    randomness : float, optional
        The scale factor by which the length is shrunken or expanded.
    """
    from contextlib import ExitStack
    from . import patheffects

    if rcParams['text.usetex']:
        raise RuntimeError(
            "xkcd mode is not compatible with text.usetex = True")

    stack = ExitStack()
    stack.callback(rcParams.update, rcParams.copy())
    rcParams.update({
        'font.family': ['xkcd', 'xkcd Script', 'Comic Neue', 'Comic Sans MS'],
        'font.size': 14.0,
        'path.sketch': (scale, length, randomness),
        'path.effects': [
            patheffects.withStroke(linewidth=4, foreground="w")],
        'axes.linewidth': 1.5,
        'lines.linewidth': 2.0,
        'figure.facecolor': 'white',
        'grid.linewidth': 0.0,
        'axes.grid': False,
        'axes.unicode_minus': False,
        'axes.edgecolor': 'black',
        'xtick.major.size': 8,
        'xtick.major.width': 3,
        'ytick.major.size': 8,
        'ytick.major.width': 3,
    })
    return stack


@_copy_docstring_and_deprecators(Figure.clear)
def clf():
    _gcf().clear()


@_copy_docstring_and_deprecators(Figure.legend)
def figlegend(*args, **kwargs):
    return _gcf().legend(*args, **kwargs)


@_copy_docstring_and_deprecators(Figure.figimage)
def figimage(X, xo=0, yo=0, alpha=None, norm=None, cmap=None, vmin=None,
             vmax=None, origin=None, resize=False, *, colorizer=None,
             **kwargs):
    return _gcf().figimage(
        X, xo=xo, yo=yo, alpha=alpha, norm=norm, cmap=cmap, vmin=vmin,
        vmax=vmax, origin=origin, resize=resize, colorizer=colorizer,
        **kwargs)


def connect(s, func):
    """Connect *func* to the event *s* of the current figure's canvas
    (`.FigureCanvasBase.mpl_connect`); return the connection id."""
    return _gcf().canvas.mpl_connect(s, func)


def disconnect(cid):
    """Disconnect the callback *cid* from the current figure's canvas
    (`.FigureCanvasBase.mpl_disconnect`)."""
    _gcf().canvas.mpl_disconnect(cid)


@_copy_docstring_and_deprecators(Figure.ginput)
def ginput(n=1, timeout=30, show_clicks=True, mouse_add=MouseButton.LEFT,
           mouse_pop=MouseButton.RIGHT, mouse_stop=MouseButton.MIDDLE):
    return _gcf().ginput(
        n=n, timeout=timeout, show_clicks=show_clicks, mouse_add=mouse_add,
        mouse_pop=mouse_pop, mouse_stop=mouse_stop)


@_copy_docstring_and_deprecators(Figure.waitforbuttonpress)
def waitforbuttonpress(timeout=-1):
    return _gcf().waitforbuttonpress(timeout=timeout)


def subplot_tool(targetfig=None):
    """
    Launch a subplot tool window for a figure.

    Returns
    -------
    `matplotlib.widgets.SubplotTool`
    """
    if targetfig is None:
        targetfig = _gcf()
    tb = targetfig.canvas.manager.toolbar
    if hasattr(tb, "configure_subplots"):  # toolbar2
        return tb.configure_subplots()
    elif hasattr(tb, "trigger_tool"):  # toolmanager
        tb.trigger_tool("subplots")
        return None
    else:
        raise ValueError("subplot_tool can only be launched for figures with "
                         "an associated toolbar")


def get_plot_commands():
    """
    Get a sorted list of all of the plotting commands.
    """
    NON_PLOT_COMMANDS = {
        'connect', 'disconnect', 'get_current_fig_manager', 'ginput',
        'new_figure_manager', 'waitforbuttonpress'}
    return [name for name in _get_pyplot_commands()
            if name not in NON_PLOT_COMMANDS]


def _get_pyplot_commands():
    # This works by searching for all functions in pyplot and removing
    # a few hard-coded exclusions, as well as all of the colormap-setting
    # functions, and anything marked as private with a preceding underscore.
    from . import pyplot
    exclude = {'colormaps', 'colors', 'get_plot_commands', *_COLORMAPS}
    return sorted(
        name for name, obj in vars(pyplot).items()
        if not name.startswith('_') and name not in exclude
           and inspect.isfunction(obj)
           and inspect.getmodule(obj) is pyplot)


#: The colormaps pyplot has a function for, which sets it as the default
#: and on the current image.
_COLORMAPS = (
    "autumn", "bone", "cool", "copper", "flag", "gray", "hot", "hsv", "jet",
    "pink", "prism", "spring", "summer", "winter", "magma", "inferno",
    "plasma", "viridis", "nipy_spectral",
)


def _colormap_function(name):
    """The pyplot function setting the colormap *name*."""
    def set_colormap():
        from . import pyplot
        pyplot.set_cmap(name)

    set_colormap.__name__ = set_colormap.__qualname__ = name
    set_colormap.__doc__ = f"""
    Set the colormap to {name!r}.

    This changes the default colormap as well as the colormap of the current
    image if there is one. See ``help(colormaps)`` for more information.
    """
    return set_colormap


globals().update({name: _colormap_function(name) for name in _COLORMAPS})

for _function in (findobj, get, pause, xkcd, clf, figlegend, figimage, connect,
                  disconnect, ginput, waitforbuttonpress, subplot_tool,
                  get_plot_commands, install_repl_displayhook,
                  uninstall_repl_displayhook,
                  *(globals()[name] for name in _COLORMAPS)):
    _function.__module__ = __name__.rpartition(".")[0] + ".pyplot"

__all__ = [
    "clf", "connect", "disconnect", "draw_all", "figimage", "figlegend",
    "findobj", "get", "get_plot_commands", "ginput",
    "install_repl_displayhook", "pause", "subplot_tool",
    "uninstall_repl_displayhook", "waitforbuttonpress", "xkcd", *_COLORMAPS,
]
