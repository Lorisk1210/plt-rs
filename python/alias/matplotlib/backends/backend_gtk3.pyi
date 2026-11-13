# The type stub of matplotlib.backends.backend_gtk3: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import logging
import os
from pathlib import Path
import matplotlib as mpl
from matplotlib import _api, backend_tools, cbook
from matplotlib.backend_bases import ToolContainerBase, MouseButton, CloseEvent, KeyEvent, LocationEvent, MouseEvent, ResizeEvent
try:
    from gi import require_version as gi_require_version
except ImportError as err:
    raise ImportError('The GTK3 backends require PyGObject') from err
try:
    gi_require_version('Gtk', '3.0')
    gi_require_version('Gdk', '3.0')
except ValueError as e:
    raise ImportError(e) from e
from gi.repository import Gio, GLib, GObject, Gtk, Gdk
from . import _backend_gtk
from ._backend_gtk import _BackendGTK, _FigureCanvasGTK, _FigureManagerGTK, _NavigationToolbar2GTK, TimerGTK as TimerGTK3
_log = logging.getLogger(__name__)

@functools.cache
def _mpl_to_gtk_cursor(mpl_cursor):
    ...

class FigureCanvasGTK3(_FigureCanvasGTK, Gtk.DrawingArea):
    required_interactive_framework = 'gtk3'
    manager_class = _api.classproperty(lambda cls: FigureManagerGTK3)
    event_mask = Gdk.EventMask.BUTTON_PRESS_MASK | Gdk.EventMask.BUTTON_RELEASE_MASK | Gdk.EventMask.EXPOSURE_MASK | Gdk.EventMask.KEY_PRESS_MASK | Gdk.EventMask.KEY_RELEASE_MASK | Gdk.EventMask.ENTER_NOTIFY_MASK | Gdk.EventMask.LEAVE_NOTIFY_MASK | Gdk.EventMask.POINTER_MOTION_MASK | Gdk.EventMask.SCROLL_MASK

    def __init__(self, figure=None):
        self._idle_draw_id = 0
        self._rubberband_rect = None

    def destroy(self):
        ...

    def set_cursor(self, cursor):
        ...

    def _mpl_coords(self, event=None):
        ...

    def scroll_event(self, widget, event):
        ...

    def button_press_event(self, widget, event):
        ...

    def button_release_event(self, widget, event):
        ...

    def key_press_event(self, widget, event):
        ...

    def key_release_event(self, widget, event):
        ...

    def motion_notify_event(self, widget, event):
        ...

    def enter_notify_event(self, widget, event):
        ...

    def leave_notify_event(self, widget, event):
        ...

    def size_allocate(self, widget, allocation):
        ...

    @staticmethod
    def _mpl_buttons(event_state):
        ...

    @staticmethod
    def _mpl_modifiers(event_state, *, exclude=None):
        ...

    def _get_key(self, event):
        ...

    def _update_device_pixel_ratio(self, *args, **kwargs):
        ...

    def configure_event(self, widget, event):
        ...

    def _draw_rubberband(self, rect):
        self._rubberband_rect = rect

    def _post_draw(self, widget, ctx):
        ...

    def on_draw_event(self, widget, ctx):
        ...

    def draw(self):
        ...

    def draw_idle(self):
        self._idle_draw_id = GLib.idle_add(idle_draw)

    def flush_events(self):
        ...

class NavigationToolbar2GTK3(_NavigationToolbar2GTK, Gtk.Toolbar):

    def __init__(self, canvas):
        self._gtk_ids = {}
        self.message = Gtk.Label()
        self._gtk_ids[text] = button = Gtk.ToggleToolButton() if callback in ['zoom', 'pan'] else Gtk.ToolButton()

    def save_figure(self, *args):
        ...

class ToolbarGTK3(ToolContainerBase, Gtk.Box):
    _icon_extension = '-symbolic.svg'

    def __init__(self, toolmanager):
        self._message = Gtk.Label()
        self._groups = {}
        self._toolitems = {}

    def add_toolitem(self, name, group, position, image_file, description, toggle):
        ...

    def _add_button(self, button, group, position):
        self._groups[group] = toolbar

    def _call_tool(self, btn, name):
        ...

    def toggle_toolitem(self, name, toggled):
        ...

    def remove_toolitem(self, name):
        ...

    def _add_separator(self):
        ...

    def set_message(self, s):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK3)
class SaveFigureGTK3(backend_tools.SaveFigureBase):

    def trigger(self, *args, **kwargs):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK3)
class HelpGTK3(backend_tools.ToolHelpBase):

    def _normalize_shortcut(self, key):
        ...

    def _is_valid_shortcut(self, key):
        ...

    def _show_shortcuts_window(self):
        ...

    def _show_shortcuts_dialog(self):
        ...

    def trigger(self, *args):
        ...

@backend_tools._register_tool_class(FigureCanvasGTK3)
class ToolCopyToClipboardGTK3(backend_tools.ToolCopyToClipboardBase):

    def trigger(self, *args, **kwargs):
        ...
Toolbar = ToolbarGTK3
backend_tools._register_tool_class(FigureCanvasGTK3, _backend_gtk.ConfigureSubplotsGTK)
backend_tools._register_tool_class(FigureCanvasGTK3, _backend_gtk.RubberbandGTK)

class FigureManagerGTK3(_FigureManagerGTK):
    _toolbar2_class = NavigationToolbar2GTK3
    _toolmanager_toolbar_class = ToolbarGTK3

@_BackendGTK.export
class _BackendGTK3(_BackendGTK):
    FigureCanvas = FigureCanvasGTK3
    FigureManager = FigureManagerGTK3
__all__ = ['functools', 'logging', 'os', 'Path', 'mpl', 'backend_tools', 'cbook', 'ToolContainerBase', 'MouseButton', 'CloseEvent', 'KeyEvent', 'LocationEvent', 'MouseEvent', 'ResizeEvent', 'Gio', 'GLib', 'GObject', 'Gtk', 'Gdk', 'TimerGTK3', 'FigureCanvasGTK3', 'NavigationToolbar2GTK3', 'ToolbarGTK3', 'SaveFigureGTK3', 'HelpGTK3', 'ToolCopyToClipboardGTK3', 'Toolbar', 'FigureManagerGTK3']
