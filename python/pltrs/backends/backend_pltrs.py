"""
pltrs' own windows as a matplotlib backend (``pltrs``, and ``macosx`` on
macOS): figures drawn by pltrs (Rust)'s Agg canvas, shown in the windows
pltrs (Rust) opens on X11, Wayland, macOS and Windows, with pltrs' toolbar
in the chrome of its look (`.look`).

The windows only show and report (`._pltrs_loop`); everything else is
matplotlib's, as with any GUI toolkit: input becomes matplotlib's events
here (`FigureCanvasPltrs`), the toolbar is a `.NavigationToolbar2` or a
`.ToolContainerBase` over pltrs' toolbar, the manager is a
`.FigureManagerBase`, and timers are `.TimerBase`\\ s. Saving from the
toolbar asks where with pltrs' file chooser, then saves with
`.Figure.savefig`. The subplot configuration tool is pltrs' dialog
(`SubplotToolPltrs`), as Qt's backend has a dialog of its own; it adjusts
constrained layout too.
"""

import os
import pathlib

import numpy as np
from PIL import Image

from .. import _api, backend_tools, cbook, is_interactive, rcParams
from .._pylab_helpers import Gcf
from ..backend_bases import (
    _Backend, CloseEvent, FigureManagerBase, KeyEvent, LocationEvent,
    MouseEvent, NavigationToolbar2, ResizeEvent, TimerBase, ToolContainerBase,
    _Mode)
from ..layout_engine import (
    ConstrainedLayoutEngine, PlaceHolderLayoutEngine, TightLayoutEngine)
from .._native import backends as _backends, gui as _gui
from . import _pltrs_loop
from .backend_agg import FigureCanvasAgg

#: The side of the bitmaps made of the program's toolbar images, in
#: pixels: twice the 24 logical pixels a button shows, for 2× screens.
_BITMAP_SIDE = 48


def _tool_picture(image_file):
    """
    The picture of a toolbar button showing *image_file* (a name of
    matplotlib's toolbar images, or a path): ``(name, None)`` for one of
    the images pltrs draws itself, in the chrome of its look, else
    ``(None, bitmap)``, the image read by Pillow as a square RGBA array.
    """
    path = pathlib.Path(image_file)
    if not path.is_absolute():
        path = cbook._get_data_path("images", f"{image_file}.png")
    name = path.stem.removesuffix("_large")
    if (path.parent == cbook._get_data_path("images")
            and _gui.has_tool_image(name)):
        return name, None
    with Image.open(path) as image:
        square = image.convert("RGBA").resize((_BITMAP_SIDE, _BITMAP_SIDE))
        return None, np.asarray(square)


#: The subplot parameters in the order the subplot tool's layouts hold
#: them.
_SUBPLOT_PARAMS = ("left", "bottom", "right", "top", "wspace", "hspace")
#: Constrained layout's parameters in the order the subplot tool's
#: layouts hold them.
_CONSTRAINED_PARAMS = ("h_pad", "w_pad", "hspace", "wspace")


class SubplotToolPltrs:
    """
    The subplot configuration tool of a pltrs window, which
    `NavigationToolbar2Pltrs.configure_subplots` returns (as Qt's returns
    its ``SubplotToolQt``): pltrs' dialog with a slider and a field per
    subplot parameter and the Qt tool's buttons, or, while constrained
    layout places the subplots (pltrs' modern looks), its padding and
    spacing, with a switch to the subplot parameters.

    The dialog reports what the user asks for; the tool carries it out
    with matplotlib's calls (`.Figure.subplots_adjust`,
    `.Figure.tight_layout`, `.ConstrainedLayoutEngine.set`) and shows the
    figure's layout in the dialog again.
    """

    def __init__(self, targetfig, window):
        self._figure = targetfig
        self._window = window
        #: What "Reset" puts back: the engine, its parameters and the
        #: subplot parameters.
        self._defaults = None

    def update_from_current_subplotpars(self):
        """
        Show the figure's layout as it is now, which "Reset" puts back,
        opening or raising the dialog.
        """
        fig = self._figure
        engine = fig.get_layout_engine()
        self._defaults = (
            engine,
            engine.get() if isinstance(engine, ConstrainedLayoutEngine) else None,
            {name: getattr(fig.subplotpars, name) for name in _SUBPLOT_PARAMS})
        offer = (isinstance(engine, ConstrainedLayoutEngine)
                 or rcParams["figure.constrained_layout.use"])
        _gui.open_subplot_tool(_pltrs_loop.host(), self._window,
                               self._layout(), offer)

    def _layout(self):
        """The figure's layout as the dialog takes it."""
        engine = self._figure.get_layout_engine()
        if isinstance(engine, ConstrainedLayoutEngine):
            params = engine.get()
            return "constrained", tuple(float(params[name])
                                        for name in _CONSTRAINED_PARAMS)
        pars = self._figure.subplotpars
        return "params", tuple(float(getattr(pars, name))
                               for name in _SUBPLOT_PARAMS)

    def _apply(self, kind, values):
        """
        Carry out what the dialog asked for (*kind*, with *values* in the
        order of the dialog's layouts), then show the figure's layout in
        it, with the reason when the figure refused.
        """
        fig = self._figure
        problem = ""
        try:
            if kind == "adjust":
                self._let_params_be()
                fig.subplots_adjust(**dict(zip(_SUBPLOT_PARAMS, values)))
            elif kind == "constrain":
                self._constrain(dict(zip(_CONSTRAINED_PARAMS, values)))
            elif kind == "tight_layout":
                self._let_params_be()
                fig.tight_layout()
            elif kind == "reset":
                self._reset()
        except (RuntimeError, ValueError) as error:
            problem = str(error)
        if self._window is not None:
            _gui.show_subplot_layout(_pltrs_loop.host(), self._window,
                                     self._layout(), problem)
        fig.canvas.draw_idle()

    def _let_params_be(self):
        """
        Replace an engine placing the subplots at every draw (constrained
        or tight layout) by a placeholder letting the subplot parameters
        be, keeping its way of making colorbars, and put the subplots back
        where the parameters place them (a tight layout measures them
        there).
        """
        fig = self._figure
        engine = fig.get_layout_engine()
        if isinstance(engine, (ConstrainedLayoutEngine, TightLayoutEngine)):
            fig.set_layout_engine(
                PlaceHolderLayoutEngine(True, engine.colorbar_gridspec))
            fig.subplots_adjust(**{name: getattr(fig.subplotpars, name)
                                   for name in _SUBPLOT_PARAMS})

    def _constrain(self, spacing):
        """Lay the figure out by constrained layout with *spacing*, keeping
        the rest of the engine it has, or had when the tool opened."""
        fig = self._figure
        engine = fig.get_layout_engine()
        if not isinstance(engine, ConstrainedLayoutEngine):
            saved, saved_params, _ = self._defaults
            if isinstance(saved, ConstrainedLayoutEngine):
                engine = ConstrainedLayoutEngine(
                    rect=saved_params["rect"], compress=saved._compress)
            else:
                engine = ConstrainedLayoutEngine()
            fig.set_layout_engine(engine)
        engine.set(**spacing)

    def _reset(self):
        """Put back the layout the figure had when the tool opened."""
        fig = self._figure
        engine, engine_params, params = self._defaults
        if isinstance(engine, ConstrainedLayoutEngine):
            fig.set_layout_engine(engine)
            engine.set(**{name: engine_params[name]
                          for name in (*_CONSTRAINED_PARAMS, "rect")})
            return
        self._let_params_be()
        fig.subplots_adjust(**params)
        if engine is not None and fig.get_layout_engine() is not engine:
            fig.set_layout_engine(engine)


class TimerPltrs(TimerBase):
    """`.TimerBase` firing in the windows' event loop."""

    def __init__(self, *args, **kwargs):
        self._timer = None
        super().__init__(*args, **kwargs)

    def _timer_start(self):
        self._timer_stop()
        self._timer = _pltrs_loop.start_timer(
            self, self._interval / 1000, not self._single)

    def _timer_stop(self):
        if self._timer is not None:
            _pltrs_loop.stop_timer(self._timer)
        self._timer = None

    def _timer_set_interval(self):
        if self._timer is not None:
            self._timer_start()

    def _timer_set_single_shot(self):
        if self._timer is not None:
            self._timer_start()

    def _fired(self, number):
        """The loop's call when the host's timer *number* fired."""
        if number != self._timer:
            return
        if self._single:
            # The host forgets a single shot once it fired; the callbacks
            # may start the timer again.
            _pltrs_loop.stop_timer(number)
            self._timer = None
        self._on_timer()


class FigureCanvasPltrs(FigureCanvasAgg):
    """
    The canvas of a pltrs window: draws with pltrs (Rust)'s Agg canvas,
    which the window shows, and turns the window's input into matplotlib's
    events.
    """

    required_interactive_framework = _pltrs_loop.FRAMEWORK
    manager_class = _api.classproperty(lambda cls: FigureManagerPltrs)
    _timer_cls = TimerPltrs

    def __init__(self, figure=None):
        super().__init__(figure)
        #: The number of the window showing the canvas, set by its manager.
        self._window = None
        self._looping = False
        #: The subplot tool, once opened (`SubplotToolPltrs`).
        self._subplot_tool = None

    def draw(self):
        # docstring inherited
        super().draw()
        self._present()

    def blit(self, bbox=None):
        # docstring inherited
        # The window shows the blitted region of the canvas (matplotlib's
        # Agg canvas has no blit of its own: the toolkits' canvases, which
        # put it before their widget's class, blit with the widget's).
        _backends.blit(self.figure._native,
                       None if bbox is None else tuple(map(float, bbox.extents)))
        self._present()

    def _present(self):
        """Have the window show what changed on the canvas."""
        if self._window is not None:
            _gui.present(_pltrs_loop.host(), self._window)

    def draw_idle(self):
        # docstring inherited
        if not self._is_idle_drawing:
            _pltrs_loop.draw_idle(self)

    def _draw_idle_now(self):
        """The idle draw `draw_idle` asked for, as the loop runs it."""
        with self._idle_draw_cntx():
            self.draw()

    def flush_events(self):
        # docstring inherited
        _pltrs_loop.process_events(0)

    def start_event_loop(self, timeout=0):
        # docstring inherited
        self._looping = True
        _pltrs_loop.run(lambda: self._looping,
                        timeout if timeout > 0 else None)

    def stop_event_loop(self):
        # docstring inherited
        self._looping = False

    def set_cursor(self, cursor):
        # docstring inherited
        if self._window is not None:
            _gui.set_cursor(_pltrs_loop.host(), self._window,
                            backend_tools.Cursors(cursor).name.lower())

    def _mpl_coords(self, x, y):
        """matplotlib's coordinates of a point of the window's canvas
        (physical pixels from its top-left corner): from the figure's
        bottom, as Tk's and Qt's backends count them."""
        if x is None:
            return None, None
        return x, self.figure.bbox.height - y

    def _handle_window_event(self, event):
        """Turn one event of the window into what matplotlib does with it."""
        kind = event[0]
        if kind == "resize":
            self._resize(*event[2:])
            return
        if kind == "tool":
            toolbar = self.toolbar or getattr(self.manager, "toolbar", None)
            if toolbar is not None:
                toolbar._clicked(event[2])
            return
        if kind == "close":
            if self.manager is not None:
                Gcf.destroy(self.manager)
            return
        if kind == "subplots":
            if self._subplot_tool is not None:
                self._subplot_tool._apply(*event[2:])
            return
        _, _, x, y, modifiers, buttons, detail = event
        x, y = self._mpl_coords(x, y)
        if kind == "motion":
            MouseEvent("motion_notify_event", self, x, y, buttons=buttons,
                       modifiers=modifiers)._process()
        elif kind == "enter":
            LocationEvent("figure_enter_event", self, x, y,
                          modifiers=modifiers)._process()
        elif kind == "leave":
            LocationEvent("figure_leave_event", self, x, y,
                          modifiers=modifiers)._process()
        elif kind == "press":
            button, dblclick = detail
            MouseEvent("button_press_event", self, x, y, button,
                       dblclick=dblclick, modifiers=modifiers)._process()
        elif kind == "release":
            MouseEvent("button_release_event", self, x, y, detail,
                       modifiers=modifiers)._process()
        elif kind == "scroll":
            MouseEvent("scroll_event", self, x, y, step=detail,
                       modifiers=modifiers)._process()
        elif kind == "key_press":
            KeyEvent("key_press_event", self, detail, x, y)._process()
        elif kind == "key_release":
            KeyEvent("key_release_event", self, detail, x, y)._process()

    def _resize(self, width, height, scale_factor):
        """The window's canvas is *width* × *height* physical pixels at
        *scale_factor*: the figure takes that size at that density, as Qt's
        canvas does in its ``resizeEvent``."""
        self._set_device_pixel_ratio(scale_factor)
        dpi = self.figure.dpi
        self.figure.set_size_inches(width / dpi, height / dpi, forward=False)
        ResizeEvent("resize_event", self)._process()
        self.draw_idle()


class FigureManagerPltrs(FigureManagerBase):
    """
    The manager of a pltrs window, which opens when first shown.

    Attributes
    ----------
    canvas : `FigureCanvasPltrs`
    num : int or str
        The figure number.
    toolbar : `NavigationToolbar2Pltrs` or `ToolbarPltrs`
        The toolbar, if any.
    """

    def __init__(self, canvas, num):
        toolbar = rcParams["toolbar"] in ("toolbar2", "toolmanager")
        self._window = _pltrs_loop.add_window(
            canvas, "", canvas.get_width_height(), toolbar)
        canvas._window = self._window
        super().__init__(canvas, num)
        if is_interactive():
            self.show()
            self.canvas.draw_idle()

    @classmethod
    def start_main_loop(cls):
        # docstring inherited
        _pltrs_loop.run(_pltrs_loop.windows_shown)

    def show(self):
        # docstring inherited
        if self._window is None:
            return
        if _gui.is_shown(_pltrs_loop.host(), self._window):
            self.canvas.draw_idle()
        _gui.show(_pltrs_loop.host(), self._window)

    def destroy(self, *args):
        # docstring inherited
        if self._window is None:
            return
        CloseEvent("close_event", self.canvas)._process()
        _pltrs_loop.close(self._window)
        self._window = self.canvas._window = None
        super().destroy()

    def full_screen_toggle(self):
        # docstring inherited
        if self._window is not None:
            host = _pltrs_loop.host()
            _gui.set_fullscreen(host, self._window,
                                not _gui.is_fullscreen(host, self._window))

    def resize(self, width, height):
        # docstring inherited
        if self._window is not None:
            _gui.resize(_pltrs_loop.host(), self._window, int(width),
                        int(height))

    def get_window_title(self):
        # docstring inherited
        if self._window is None:
            return self._window_title
        return _gui.title(_pltrs_loop.host(), self._window)

    def set_window_title(self, title):
        # docstring inherited
        self._window_title = title
        if self._window is not None:
            _gui.set_title(_pltrs_loop.host(), self._window, title)


class NavigationToolbar2Pltrs(NavigationToolbar2):
    """`.NavigationToolbar2` over the toolbar of a pltrs window: a button
    per item of `toolitems`, the items between ``None`` separators in a
    group, and the coordinates in the status line."""

    def __init__(self, canvas):
        self._callbacks = {}
        group = 0
        for text, tooltip_text, image_file, callback in self.toolitems:
            if text is None:
                group += 1
                continue
            _gui.add_tool(_pltrs_loop.host(), canvas._window, text,
                          f"group{group}", None, tooltip_text,
                          *_tool_picture(image_file))
            self._callbacks[text] = callback
        super().__init__(canvas)

    def _clicked(self, name):
        """The button *name* was clicked: run its method."""
        getattr(self, self._callbacks[name])()

    def _update_buttons_checked(self):
        """Check the button of the navigation mode on."""
        for text, mode in [("Zoom", _Mode.ZOOM), ("Pan", _Mode.PAN)]:
            if text in self._callbacks:
                _gui.set_tool_checked(_pltrs_loop.host(), self.canvas._window,
                                      text, self.mode == mode)

    def pan(self, *args):
        super().pan(*args)
        self._update_buttons_checked()

    def zoom(self, *args):
        super().zoom(*args)
        self._update_buttons_checked()

    def set_message(self, s):
        if self.canvas._window is not None:
            _gui.set_message(_pltrs_loop.host(), self.canvas._window, s)

    def draw_rubberband(self, event, x0, y0, x1, y1):
        if self.canvas._window is not None:
            height = self.canvas.figure.bbox.height
            _gui.set_rubberband(_pltrs_loop.host(), self.canvas._window,
                                (x0, height - y0, x1, height - y1))

    def remove_rubberband(self):
        if self.canvas._window is not None:
            _gui.set_rubberband(_pltrs_loop.host(), self.canvas._window, None)

    def set_history_buttons(self):
        callbacks = getattr(self, "_callbacks", {})
        if self.canvas._window is None:
            return
        can_back = self._nav_stack._pos > 0
        can_forward = self._nav_stack._pos < len(self._nav_stack) - 1
        for text, enabled in [("Back", can_back), ("Forward", can_forward)]:
            if text in callbacks:
                _gui.set_tool_enabled(_pltrs_loop.host(), self.canvas._window,
                                      text, enabled)

    def configure_subplots(self, *args):
        """
        Open pltrs' subplot configuration tool, or show it again with the
        figure's layout as it is now; return it (`SubplotToolPltrs`).
        """
        canvas = self.canvas
        if canvas._window is None:
            return None
        # The tool belongs to the figure's window, shown with it as the
        # figure it adjusts (``plt.subplot_tool()`` before ``plt.show()``).
        if not _gui.is_shown(_pltrs_loop.host(), canvas._window):
            canvas.manager.show()
        if canvas._subplot_tool is None:
            canvas._subplot_tool = SubplotToolPltrs(canvas.figure,
                                                    canvas._window)
        canvas._subplot_tool.update_from_current_subplotpars()
        return canvas._subplot_tool

    def save_figure(self, *args):
        """
        Ask where to save the figure with pltrs' file chooser and save it
        there; return the file name, or None when cancelled. The chosen
        directory becomes :rc:`savefig.directory` unless it is empty.
        """
        if self.canvas._window is None:
            return None
        fname = _pltrs_loop.choose_file(self.canvas._window,
                                        self.canvas.get_default_filename())
        if fname is None:
            return None
        if os.path.expanduser(rcParams["savefig.directory"]) != "":
            rcParams["savefig.directory"] = os.path.dirname(fname)
        try:
            self.canvas.figure.savefig(fname)
        except Exception as e:
            self.set_message(f"Error saving file: {e}")
        return fname


class ToolbarPltrs(ToolContainerBase):
    """`.ToolContainerBase` over the toolbar of a pltrs window, for
    :rc:`toolbar` ``toolmanager``."""

    @property
    def _window(self):
        return self.toolmanager.canvas._window

    def add_toolitem(self, name, group, position, image_file, description,
                     toggle):
        picture = (_tool_picture(image_file) if image_file is not None
                   else (name, None))
        _gui.add_tool(_pltrs_loop.host(), self._window, name, group,
                      None if position < 0 else position, description or "",
                      *picture)

    def _clicked(self, name):
        """The button *name* was clicked: trigger its tool."""
        self.trigger_tool(name)

    def toggle_toolitem(self, name, toggled):
        _gui.set_tool_checked(_pltrs_loop.host(), self._window, name, toggled)

    def remove_toolitem(self, name):
        _gui.remove_tool(_pltrs_loop.host(), self._window, name)

    def set_message(self, s):
        if self._window is not None:
            _gui.set_message(_pltrs_loop.host(), self._window, s)


@backend_tools._register_tool_class(FigureCanvasPltrs)
class SaveFigurePltrs(backend_tools.SaveFigureBase):
    def trigger(self, *args):
        NavigationToolbar2Pltrs.save_figure(
            self._make_classic_style_pseudo_toolbar())


@backend_tools._register_tool_class(FigureCanvasPltrs)
class ConfigureSubplotsPltrs(backend_tools.ConfigureSubplotsBase):
    def trigger(self, *args):
        NavigationToolbar2Pltrs.configure_subplots(
            self._make_classic_style_pseudo_toolbar())


@backend_tools._register_tool_class(FigureCanvasPltrs)
class RubberbandPltrs(backend_tools.RubberbandBase):
    def draw_rubberband(self, x0, y0, x1, y1):
        NavigationToolbar2Pltrs.draw_rubberband(
            self._make_classic_style_pseudo_toolbar(), None, x0, y0, x1, y1)

    def remove_rubberband(self):
        NavigationToolbar2Pltrs.remove_rubberband(
            self._make_classic_style_pseudo_toolbar())


@backend_tools._register_tool_class(FigureCanvasPltrs)
class HelpPltrs(backend_tools.ToolHelpBase):
    def trigger(self, *args):
        rows = [(description, keys.split(", ") if keys else [])
                for _, keys, description in self._get_help_entries()]
        _gui.show_help(_pltrs_loop.host(), self.toolmanager.canvas._window,
                       [("Tools", rows)])


@backend_tools._register_tool_class(FigureCanvasPltrs)
class ToolCopyToClipboardPltrs(backend_tools.ToolCopyToClipboardBase):
    def trigger(self, *args, **kwargs):
        _gui.copy_to_clipboard(_pltrs_loop.host(),
                               self.toolmanager.canvas._window)


FigureManagerPltrs._toolbar2_class = NavigationToolbar2Pltrs
FigureManagerPltrs._toolmanager_toolbar_class = ToolbarPltrs


@_Backend.export
class _BackendPltrs(_Backend):
    backend_version = "pltrs"
    FigureCanvas = FigureCanvasPltrs
    FigureManager = FigureManagerPltrs
    mainloop = FigureManagerPltrs.start_main_loop
