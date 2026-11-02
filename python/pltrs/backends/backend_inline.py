"""
Jupyter's inline backend for pltrs (``inline``, and
``module://matplotlib_inline.backend_inline``, which IPython's kernel
selects): figures shown as images in a notebook's output once each cell
has run, for ``import pltrs.pyplot as plt`` in an IPython kernel.

matplotlib-inline and the parts of IPython it calls import ``matplotlib``
(``activate_matplotlib``, ``select_figure_formats``), so this is their
port over pltrs' own names: the kernel's display formatters show pltrs'
`.Figure`\\ s as PNG (or the formats `InlineBackend.figure_formats`
names), drawn by IPython's ``print_figure`` through the figure's
canvas, and `flush_figures` runs after each cell (`configure_inline_support`).
Where pltrs is installed as ``matplotlib``, matplotlib-inline itself
serves (`.registry` loads it for the same names).

From matplotlib-inline 0.2.2 (``backend_inline`` and ``config``) and
IPython 9 (``pylabtools.select_figure_formats`` and
``activate_matplotlib``), under the BSD 3-Clause License, Copyright (c)
IPython Development Team.
"""

from functools import partial

from IPython.core.getipython import get_ipython
from IPython.core.interactiveshell import InteractiveShell
from IPython.core.pylabtools import print_figure, retina_figure
from IPython.display import display
from IPython.utils.decorators import flag_calls
from traitlets import Bool, Dict, Instance, Set, TraitError, Unicode
from traitlets.config.configurable import SingletonConfigurable

from .. import colors, get_backend, interactive, is_interactive, rcParams
from .._pylab_helpers import Gcf
from ..figure import Figure
from . import backend_agg
from .backend_agg import FigureCanvasAgg

#: The names this backend is selected under.
_INLINE_NAMES = ("inline", "module://matplotlib_inline.backend_inline")


def pil_available():
    """Test if PIL/Pillow is available"""
    try:
        from PIL import Image  # noqa: F401
    except ImportError:
        return False
    return True


class InlineBackendConfig(SingletonConfigurable):
    """The configurable `InlineBackend` derives from (for configurations
    naming the old class)."""


class InlineBackend(InlineBackendConfig):
    """An object to store configuration of the inline backend."""

    rc = Dict(
        {},
        help="""Dict to manage pltrs' configuration defaults in the inline
        backend. To change personal defaults, use pltrs' configuration
        tools, or customize this class in your `ipython_config.py` file for
        IPython/Jupyter-specific usage.""",
    ).tag(config=True)

    figure_formats = Set(
        {"png"},
        help="""A set of figure formats to enable: 'png',
                'retina', 'jpeg', 'svg', 'pdf'.""",
    ).tag(config=True)

    def _update_figure_formatters(self):
        if self.shell is not None:
            select_figure_formats(
                self.shell, self.figure_formats, **self.print_figure_kwargs)

    def _figure_formats_changed(self, name, old, new):
        if "jpg" in new or "jpeg" in new:
            if not pil_available():
                raise TraitError("Requires PIL/Pillow for JPG figures")
        self._update_figure_formatters()

    figure_format = Unicode(
        help="""The figure format to enable (deprecated
                                         use `figure_formats` instead)"""
    ).tag(config=True)

    def _figure_format_changed(self, name, old, new):
        if new:
            self.figure_formats = {new}

    print_figure_kwargs = Dict(
        {"bbox_inches": "tight"},
        help="""Extra kwargs to be passed to fig.canvas.print_figure.

        Logical examples include: bbox_inches, pil_kwargs, etc. In addition,
        see the docstrings of `set_matplotlib_formats`.
        """,
    ).tag(config=True)
    _print_figure_kwargs_changed = _update_figure_formatters

    close_figures = Bool(
        True,
        help="""Close all figures at the end of each cell.

        When True, ensures that each cell starts with no active figures, but
        it also means that one must keep track of references in order to edit
        or redraw figures in subsequent cells.

        When False, one must call figure() to create new figures, and gcf()
        can reference figures created in other cells.
        """,
    ).tag(config=True)

    shell = Instance(
        "IPython.core.interactiveshell.InteractiveShellABC", allow_none=True)


def select_figure_formats(shell, formats, **kwargs):
    """
    Show pltrs' figures in *shell*'s output in the figure formats
    *formats* (one or a set of 'png', 'retina', 'jpeg', 'svg', 'pdf'),
    drawn with the keyword arguments *kwargs* of ``print_figure``
    (IPython's ``select_figure_formats``, for pltrs' figures).
    """
    formatters = shell.display_formatter.formatters
    if isinstance(formats, str):
        formats = {formats}
    formats = set(formats)
    for formatter in formatters.values():
        formatter.pop(Figure, None)
    supported = {'png', 'png2x', 'retina', 'jpg', 'jpeg', 'svg', 'pdf'}
    bad = formats.difference(supported)
    if bad:
        bs = "%s" % ','.join([repr(f) for f in bad])
        gs = "%s" % ','.join([repr(f) for f in supported])
        raise ValueError("supported formats are: {} not {}".format(gs, bs))
    if "png" in formats:
        formatters['image/png'].for_type(
            Figure, partial(print_figure, fmt="png", base64=True, **kwargs))
    if "retina" in formats or "png2x" in formats:
        formatters['image/png'].for_type(
            Figure, partial(retina_figure, base64=True, **kwargs))
    if "jpg" in formats or "jpeg" in formats:
        formatters['image/jpeg'].for_type(
            Figure, partial(print_figure, fmt="jpg", base64=True, **kwargs))
    if "svg" in formats:
        formatters['image/svg+xml'].for_type(
            Figure, partial(print_figure, fmt="svg", **kwargs))
    if "pdf" in formats:
        formatters['application/pdf'].for_type(
            Figure, partial(print_figure, fmt="pdf", base64=True, **kwargs))


def activate_matplotlib(backend):
    """Switch pyplot to *backend* and to interactive mode, flagging its
    draws for `flush_figures` (IPython's ``activate_matplotlib``)."""
    interactive(True)
    # The parameter is set *before* switch_backend's module magic.
    rcParams['backend'] = backend
    from .. import pyplot as plt
    plt.switch_backend(backend)
    plt.show._needmain = False
    # Detect at runtime whether show() is called by the user.
    plt.draw_if_interactive = flag_calls(plt.draw_if_interactive)


def new_figure_manager(num, *args, FigureClass=Figure, **kwargs):
    """
    Return a new figure manager for a new figure instance.

    This function is part of the API expected by Matplotlib backends.
    """
    return new_figure_manager_given_figure(num, FigureClass(*args, **kwargs))


def new_figure_manager_given_figure(num, figure):
    """
    Return a new figure manager for a given figure instance.

    This function is part of the API expected by Matplotlib backends.
    """
    manager = backend_agg.new_figure_manager_given_figure(num, figure)

    # Hack: the managers of interactive backends monkeypatch the figure
    # with a .show() method; this does the same, for user code expecting
    # it.  (https://github.com/ipython/ipython/issues/1612,
    # https://github.com/matplotlib/matplotlib/issues/835)
    if not hasattr(figure, "show"):
        # Queue up `figure` for display
        figure.show = lambda *a: display(
            figure, metadata=_fetch_figure_metadata(figure))

    # Manually set non-interactive mode: no-op (else duplicate plots, since
    # a user who set ioff() manually expects separate draw/show calls).
    if not is_interactive():
        return manager

    # The current figure will be drawn, and each later call of
    # draw_if_interactive() moves the active figure to be drawn last.
    try:
        show._to_draw.remove(figure)
    except ValueError:
        # It only appears in the draw list once.
        pass
    # Queue up the figure for drawing in next show() call
    show._to_draw.append(figure)
    show._draw_called = True

    return manager


def show(close=None, block=None):
    """Show all figures as SVG/PNG payloads sent to the IPython clients.

    Parameters
    ----------
    close : bool, optional
        If true, a ``plt.close('all')`` call is automatically issued after
        sending all the figures. If this is set, the figures will entirely
        removed from the internal list of figures.
    block : Not used.
        The `block` parameter is a Matplotlib experimental parameter.
        We accept it in the function signature for compatibility with other
        backends.
    """
    if close is None:
        close = InlineBackend.instance().close_figures
    try:
        for figure_manager in Gcf.get_all_fig_managers():
            display(
                figure_manager.canvas.figure,
                metadata=_fetch_figure_metadata(figure_manager.canvas.figure),
            )
    finally:
        show._to_draw = []
        # Only close('all') if there is any to close: it collects garbage,
        # which can be slow.
        if close and Gcf.get_all_fig_managers():
            from .. import pyplot
            pyplot.close("all")


# This flag will be reset by draw_if_interactive when called
show._draw_called = False
# list of figures to draw when flush_figures is called
show._to_draw = []


def flush_figures():
    """Send all figures that changed

    This is meant to be called automatically and will call show() if, during
    prior code execution, there had been any calls to draw_if_interactive.

    This function is meant to be used as a post_execute callback in IPython,
    so user-caused errors are handled with showtraceback() instead of being
    allowed to raise.  If this function is not called from within IPython,
    then these exceptions will raise.
    """
    if not show._draw_called:
        return

    try:
        if InlineBackend.instance().close_figures:
            # ignore the tracking, just draw and close all figures
            try:
                return show(True)
            except Exception as e:
                # safely show traceback if in IPython, else raise
                ip = get_ipython()
                if ip is None:
                    raise e
                else:
                    ip.showtraceback()
                    return

        # exclude any figures that were closed:
        active = {fm.canvas.figure for fm in Gcf.get_all_fig_managers()}
        for fig in [fig for fig in show._to_draw if fig in active]:
            try:
                display(fig, metadata=_fetch_figure_metadata(fig))
            except Exception as e:
                # safely show traceback if in IPython, else raise
                ip = get_ipython()
                if ip is None:
                    raise e
                else:
                    ip.showtraceback()
                    return
    finally:
        # clear flags for next round
        show._to_draw = []
        show._draw_called = False


#: The canvas of the backend's figures: Agg's.
FigureCanvas = FigureCanvasAgg


def configure_inline_support(shell, backend):
    """Configure an IPython shell object for pltrs use.

    Parameters
    ----------
    shell : InteractiveShell instance

    backend : backend name
    """
    cfg = InlineBackend.instance(parent=shell)
    cfg.shell = shell
    if cfg not in shell.configurables:
        shell.configurables.append(cfg)

    if backend in _INLINE_NAMES:
        shell.events.register("post_execute", flush_figures)

        # Save rcParams that will be overwritten
        shell._saved_rcParams = {}
        for k in cfg.rc:
            shell._saved_rcParams[k] = rcParams[k]
        # load inline_rc
        rcParams.update(cfg.rc)
        new_backend_name = "inline"
    else:
        try:
            shell.events.unregister("post_execute", flush_figures)
        except ValueError:
            pass
        if hasattr(shell, "_saved_rcParams"):
            rcParams.update(shell._saved_rcParams)
            del shell._saved_rcParams
        new_backend_name = "other"

    # Enable the formats once: another "inline" selection keeps the formats
    # the user may have changed (https://github.com/ipython/ipykernel/issues/29).
    cur_backend = getattr(configure_inline_support, "current_backend", "unset")
    if new_backend_name != cur_backend:
        # Setup the default figure format
        select_figure_formats(shell, cfg.figure_formats,
                              **cfg.print_figure_kwargs)
        configure_inline_support.current_backend = new_backend_name


def _enable_matplotlib_integration():
    """Enable extra IPython integration when loaded as the backend."""
    ip = get_ipython()
    backend = get_backend(auto_select=False)
    if ip and backend in _INLINE_NAMES:
        try:
            activate_matplotlib(backend)
            configure_inline_support(ip, backend)
        except (ImportError, AttributeError):
            # A circular import while pyplot loads this module: configure
            # after the cell.
            def configure_once(*args):
                activate_matplotlib(backend)
                configure_inline_support(ip, backend)
                ip.events.unregister("post_run_cell", configure_once)

            ip.events.register("post_run_cell", configure_once)


_enable_matplotlib_integration()


def _fetch_figure_metadata(fig):
    """Get some metadata to help with displaying a figure."""
    # determine if a background is needed for legibility
    if _is_transparent(fig.get_facecolor()):
        # the background is transparent
        ticksLight = _is_light([
            label.get_color()
            for axes in fig.axes
            for axis in (axes.xaxis, axes.yaxis)
            for label in axis.get_ticklabels()
        ])
        if ticksLight.size and (ticksLight == ticksLight[0]).all():
            # there are one or more tick labels, all with the same lightness
            return {"needs_background": "dark" if ticksLight[0] else "light"}

    return None


def _is_light(color):
    """Determines if a color (or each of a sequence of colors) is light (as
    opposed to dark). Based on ITU BT.601 luminance formula (see
    https://stackoverflow.com/a/596241)."""
    rgbaArr = colors.to_rgba_array(color)
    return rgbaArr[:, :3].dot((0.299, 0.587, 0.114)) > 0.5


def _is_transparent(color):
    """Determine transparency from alpha."""
    rgba = colors.to_rgba(color)
    return rgba[3] < 0.5


def set_matplotlib_formats(*formats, **kwargs):
    """Select figure formats for the inline backend. Optionally pass quality for JPEG.

    For example, this enables PNG and JPEG output with a JPEG quality of 90%::

        In [1]: set_matplotlib_formats('png', 'jpeg',
                                       pil_kwargs={'quality': 90})

    Parameters
    ----------
    *formats : strs
        One or more figure formats to enable: 'png', 'retina', 'jpeg', 'svg', 'pdf'.
    **kwargs
        Keyword args will be relayed to ``figure.canvas.print_figure``.
    """
    # build kwargs, starting with InlineBackend config
    cfg = InlineBackend.instance()
    kw = {}
    kw.update(cfg.print_figure_kwargs)
    kw.update(**kwargs)
    shell = InteractiveShell.instance()
    select_figure_formats(shell, formats, **kw)


def set_matplotlib_close(close=True):
    """Set whether the inline backend closes all figures automatically or not.

    By default, the inline backend used in the IPython Notebook will close all
    figures automatically after each cell is run. This means that plots in
    different cells won't interfere. Sometimes, you may want to make a plot in
    one cell and then refine it in later cells. This can be accomplished by::

        In [1]: set_matplotlib_close(False)

    Parameters
    ----------
    close : bool
        Should all figures be automatically closed after each cell is run?
    """
    cfg = InlineBackend.instance()
    cfg.close_figures = close
